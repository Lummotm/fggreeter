use crate::engine::greetd::GreeterState;
use bevy::ecs::message::MessageReader;
use bevy::input::ButtonState;
use bevy::input::keyboard::Key;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

#[derive(Resource)]
pub struct InputState {
    pub user: String,
    pub command: Vec<String>,
    pub response_buffer: String,
    pub show_response: bool,
}

#[derive(Event)]
pub struct SubmitInputEvent;

/// Register the plugins for the input handling
pub fn add_plugins(app: &mut App, args: crate::args::Args) {
    app.insert_resource(InputState {
        user: args.user,
        command: args.command,
        response_buffer: String::new(),
        show_response: true,
    });

    app.add_systems(Update, handle_keystrokes.run_if(in_state(GreeterState::Input)));
    #[cfg(debug_assertions)]
    app.add_systems(Update, debug_time_speed);
}

/// Read the user input and update the input field accordingly
fn handle_keystrokes(
    keys: Res<ButtonInput<KeyCode>>,
    mut keyboard_input: MessageReader<KeyboardInput>,
    mut input_state: ResMut<InputState>,
    mut commands: Commands,
) {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);

    for keystroke in keyboard_input.read().filter(|ev| ev.state == ButtonState::Pressed) {
        match &keystroke.logical_key {
            Key::Backspace => {
                if ctrl {
                    input_state.response_buffer.clear();
                    commands.trigger(crate::engine::animation::AnimationEvent::Reset);
                } else {
                    input_state.response_buffer.pop();
                    commands.trigger(crate::engine::animation::AnimationEvent::Block);
                }
            }
            Key::Escape => {
                input_state.response_buffer.clear();
                commands.trigger(crate::engine::animation::AnimationEvent::Reset);
            }
            Key::Enter => {
                commands.trigger(SubmitInputEvent);
                commands.trigger(crate::engine::animation::AnimationEvent::Windup);
            }
            _ => {
                if let Some(text) = &keystroke.text {
                    let chars = text.chars().filter(|c| !c.is_control());
                    input_state.response_buffer.extend(chars);
                    commands.trigger(crate::engine::animation::AnimationEvent::Hit);
                }
            }
        }
    }
}

/// Debug: cycle the global time speed to inspect effects in slow motion
#[cfg(debug_assertions)]
fn debug_time_speed(
    mut keyboard_input: MessageReader<KeyboardInput>,
    mut time: ResMut<Time<Virtual>>,
    mut speed_index: Local<usize>,
) {
    const SPEEDS: [f32; 4] = [1.0, 0.25, 0.05, 0.01];

    for keystroke in keyboard_input.read() {
        if keystroke.state == ButtonState::Pressed && keystroke.key_code == KeyCode::F1 {
            *speed_index = (*speed_index + 1) % SPEEDS.len();
            time.set_relative_speed(SPEEDS[*speed_index]);
            tracing::debug!("Time speed: x{}", SPEEDS[*speed_index]);
        }
    }
}
