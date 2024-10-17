#![allow(clippy::needless_pass_by_value)] // False positives with `impl ToString`

use std::{cmp::Ordering, ops::RangeInclusive};

use egui::{emath, text, Button, CursorIcon, Key, Modifiers, NumExt, RichText, Sense, TextEdit, TextWrapMode, Ui, Widget, WidgetInfo};

use crate::*;

// ----------------------------------------------------------------------------

type Formatter<'a, T> = Box<dyn 'a + Fn(&T) -> String>;
type Validater<'a, T> = Box<dyn 'a + Fn(&str) -> Option<T>>;

// ----------------------------------------------------------------------------

/// Combined into one function (rather than two) to make it easier
/// for the borrow checker.
type GetSetValue<'a, T> = Box<dyn 'a + FnMut(Option<&'a T>) -> &'a T>;

fn get<'a, T>(get_set_value: &mut GetSetValue<'a, T>) -> &'a T {
    (get_set_value)(None)
}

fn set<'a, T>(get_set_value: &mut GetSetValue<'a, T>, value: &'a T) {
    (get_set_value)(Some(value));
}

/// A numeric value that you can change by dragging the number. More compact than a [`Slider`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut my_f32: f32 = 0.0;
/// ui.add(egui::ValidatingValue::new(&mut my_f32).speed(0.1)); //DUMMY
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct ValidatingValue<'a, T> {
    external_value: &'a mut T,
    updated_value: Option<T>,
    formatter: Formatter<'a, T>,
    validater: Validater<'a, T>,
    update_while_editing: bool, //CHECK What's this?
}

impl<'a, T: 'a> ValidatingValue<'a, T> {
    pub fn new(
        value: &'a mut T,
        formatter: impl 'a + Fn(&T) -> String,
        validater: impl 'a + Fn(&str) -> Option<T>,
    ) -> Self {
        Self {
            external_value: value,
            updated_value: None,
            formatter: Box::new(formatter),
            validater: Box::new(validater),
            update_while_editing: true,
        }
    }

    //RAINY Maybe some defaults, like were here in DragValue?

    /// Update the value on each key press when text-editing the value.
    ///
    /// Default: `true`.
    /// If `false`, the value will only be updated when user presses enter or deselects the value.
    #[inline]
    pub fn update_while_editing(mut self, update: bool) -> Self {
        self.update_while_editing = update;
        self
    }
}

impl<'a, T> Widget for ValidatingValue<'a, T> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            external_value,
            mut updated_value,
            formatter,
            validater,
            update_while_editing,
        } = self;

        // The widget has the same ID whether it's in edit or button mode.
        let id = ui.next_auto_id();

        // The following ensures that when a `ValidatingValue` receives focus,
        // it is immediately rendered in edit mode, rather than being rendered
        // in button mode for just one frame. This is important for
        // screen readers.
        let is_kb_editing = ui.memory_mut(|mem| {
            mem.interested_in_focus(id);
            mem.has_focus(id)
        });

        if ui.memory_mut(|mem| mem.gained_focus(id)) {
            ui.data_mut(|data| data.remove::<String>(id));
        }

        let old_value_text = formatter(updated_value.as_ref().unwrap_or(external_value)); //DUMMY This means the formatting *matters* - change this or at least document it
        let value = updated_value.as_ref().unwrap_or(external_value);

        let value_text = formatter(&value);

        let text_style = ui.style().drag_value_text_style.clone();

        if ui.memory(|mem| mem.lost_focus(id)) && !ui.input(|i| i.key_pressed(Key::Escape)) {
            let value_text = ui.data_mut(|data| data.remove_temp::<String>(id));
            if let Some(value_text) = value_text {
                // We were editing the value as text last frame, but lost focus.
                // Make sure we applied the last text value:
                let parsed_value = validater(&value_text);
                if let Some(parsed_value) = parsed_value {
                    updated_value = Some(parsed_value);
                }
            }
        }

        // some clones below are redundant if AccessKit is disabled
        #[allow(clippy::redundant_clone)]
        let mut response = if is_kb_editing {
            let mut value_text = ui
                .data_mut(|data| data.remove_temp::<String>(id))
                .unwrap_or_else(|| value_text.clone());
            let response = ui.add(
                TextEdit::singleline(&mut value_text)
                    .clip_text(false)
                    .horizontal_align(ui.layout().horizontal_align())
                    .vertical_align(ui.layout().vertical_align())
                    .margin(ui.spacing().button_padding)
                    .min_size(ui.spacing().interact_size)
                    .id(id)
                    .desired_width(ui.spacing().interact_size.x)
                    .font(text_style),
            );

            let update = if update_while_editing {
                // Update when the edit content has changed.
                response.changed()
            } else {
                // Update only when the edit has lost focus.
                response.lost_focus() && !ui.input(|i| i.key_pressed(Key::Escape))
            };
            if update {
                let parsed_value = validater(&value_text);
                if let Some(parsed_value) = parsed_value {
                    updated_value = Some(parsed_value);
                }
            }
            ui.data_mut(|data| data.insert_temp(id, value_text));
            response
        } else {
            //CHECK Some of the drag code was here, not sure I eliminated it all
            let button = Button::new(
                RichText::new(value_text.clone()).text_style(text_style),
            )
            .wrap_mode(TextWrapMode::Extend)
            .min_size(ui.spacing().interact_size); // TODO(emilk): find some more generic solution to `min_size`

            let mut response = ui.add(button);

            if ui.style().explanation_tooltips {
                response = response.on_hover_text(format!(
                    "Click to enter a value.",
                ));
            }

            if response.clicked() {
                ui.data_mut(|data| data.remove::<String>(id));
                ui.memory_mut(|mem| mem.request_focus(id));
                let mut state = TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
                state.cursor.set_char_range(Some(text::CCursorRange::two(
                    text::CCursor::default(),
                    text::CCursor::new(value_text.chars().count()),
                )));
                state.store(ui.ctx(), response.id);
            }

            response
        };

        response.changed = formatter(updated_value.as_ref().unwrap_or(external_value)) != old_value_text; //DITTO //DUMMY Formatter matters, suboptimal

        //DUMMY Check this stuff
        #[cfg(feature = "accesskit")]
        ui.ctx().accesskit_node_builder(response.id, |builder| {
            use accesskit::Action;
            // If either end of the range is unbounded, it's better
            // to leave the corresponding AccessKit field set to None,
            // to allow for platform-specific default behavior.
            if range.start().is_finite() {
                builder.set_min_numeric_value(*range.start());
            }
            if range.end().is_finite() {
                builder.set_max_numeric_value(*range.end());
            }
            builder.set_numeric_value_step(speed);
            builder.add_action(Action::SetValue);
            if value < *range.end() {
                builder.add_action(Action::Increment);
            }
            if value > *range.start() {
                builder.add_action(Action::Decrement);
            }
            // The name field is set to the current value by the button,
            // but we don't want it set that way on this widget type.
            builder.clear_name();
            // Always expose the value as a string. This makes the widget
            // more stable to accessibility users as it switches
            // between edit and button modes. This is particularly important
            // for VoiceOver on macOS; if the value is not exposed as a string
            // when the widget is in button mode, then VoiceOver speaks
            // the value (or a percentage if the widget has a clamp range)
            // when the widget loses focus, overriding the announcement
            // of the newly focused widget. This is certainly a VoiceOver bug,
            // but it's good to make our software work as well as possible
            // with existing assistive technology. However, if the widget
            // has a prefix and/or suffix, expose those when in button mode,
            // just as they're exposed on the screen. This triggers the
            // VoiceOver bug just described, but exposing all information
            // is more important, and at least we can avoid the bug
            // for instances of the widget with no prefix or suffix.
            //
            // The value is exposed as a string by the text edit widget
            // when in edit mode.
            if !is_kb_editing {
                let value_text = format!("{prefix}{value_text}{suffix}");
                builder.set_value(value_text);
            }
        });

        if let Some(updated_value) = updated_value {
            *external_value = updated_value;
        }

        response
    }
}
