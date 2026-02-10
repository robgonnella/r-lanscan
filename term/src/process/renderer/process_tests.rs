use std::{convert::Infallible, io, rc::Rc};

use color_eyre::eyre::eyre;
use ratatui::{
    Terminal,
    backend::{ClearType, TestBackend, WindowSize},
    prelude::{Backend, Position, Size},
};

use crate::{
    ipc::{
        message::{MainMessage, RendererMessage},
        renderer::RendererIpc,
        traits::{MockIpcReceiver, MockIpcSender},
    },
    store::{
        StateGetter, Store, action::Action, reducer::StoreReducer, state::State,
    },
};

use super::*;

/// Wraps TestBackend with a no-op Write impl so it satisfies
/// the `Backend + Write` bound on RendererProcess.
struct WritableTestBackend(TestBackend);

impl WritableTestBackend {
    fn new(width: u16, height: u16) -> Self {
        Self(TestBackend::new(width, height))
    }
}

impl io::Write for WritableTestBackend {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Backend for WritableTestBackend {
    type Error = Infallible;

    fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        self.0.draw(content)
    }

    fn hide_cursor(&mut self) -> Result<(), Self::Error> {
        self.0.hide_cursor()
    }

    fn show_cursor(&mut self) -> Result<(), Self::Error> {
        self.0.show_cursor()
    }

    fn get_cursor_position(&mut self) -> Result<Position, Self::Error> {
        self.0.get_cursor_position()
    }

    fn set_cursor_position<P: Into<Position>>(
        &mut self,
        position: P,
    ) -> Result<(), Self::Error> {
        self.0.set_cursor_position(position)
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        self.0.clear()
    }

    fn clear_region(
        &mut self,
        clear_type: ClearType,
    ) -> Result<(), Self::Error> {
        self.0.clear_region(clear_type)
    }

    fn size(&self) -> Result<Size, Self::Error> {
        self.0.size()
    }

    fn window_size(&mut self) -> Result<WindowSize, Self::Error> {
        self.0.window_size()
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Backend::flush(&mut self.0)
    }
}

struct Setup {
    renderer: RendererProcess<WritableTestBackend>,
    store: Rc<Store>,
}

fn setup(
    mock_sender: MockIpcSender<MainMessage>,
    mock_receiver: MockIpcReceiver<RendererMessage>,
) -> Setup {
    let store = Rc::new(Store::new(State::default(), StoreReducer::boxed()));
    let backend = WritableTestBackend::new(80, 24);
    let terminal = Terminal::new(backend).unwrap();
    let ipc = RendererIpc::new(Box::new(mock_sender), Box::new(mock_receiver));
    let renderer = RendererProcess::new(terminal, ipc, store.clone());
    Setup { renderer, store }
}

#[test]
fn renders_frame_with_default_state() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mock_receiver = MockIpcReceiver::<RendererMessage>::new();
    let test = setup(mock_sender, mock_receiver);
    let state = test.store.get_state();

    let result = test.renderer.render_frame(&state);
    assert!(result.is_ok());
}

#[test]
fn skips_render_when_ui_paused() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mock_receiver = MockIpcReceiver::<RendererMessage>::new();
    let test = setup(mock_sender, mock_receiver);

    test.store.dispatch(Action::SetUIPaused(true));
    let state = test.store.get_state();
    assert!(state.ui_paused);

    let result = test.renderer.render_frame(&state);
    assert!(result.is_ok());

    // buffer should be empty since render was skipped
    let backend = test.renderer.terminal.borrow();
    let buf = backend.backend().0.buffer();
    let has_content = buf
        .content()
        .iter()
        .any(|cell| cell.symbol() != " " && !cell.symbol().is_empty());
    assert!(!has_content);
}

#[test]
fn render_frame_draws_content() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mock_receiver = MockIpcReceiver::<RendererMessage>::new();
    let test = setup(mock_sender, mock_receiver);

    test.store
        .dispatch(Action::UpdateMessage(Some("scan complete".into())));
    let state = test.store.get_state();

    test.renderer.render_frame(&state).unwrap();

    let backend = test.renderer.terminal.borrow();
    let buf = backend.backend().0.buffer();
    let content: String =
        buf.content().iter().map(|cell| cell.symbol()).collect();
    assert!(content.contains("scan complete"));
}

#[test]
fn paused_loop_dispatches_action_sync() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<RendererMessage>::new();

    mock_receiver
        .expect_recv()
        .returning(|| {
            Ok(RendererMessage::ActionSync(Box::new(Action::SetError(
                Some("test error".into()),
            ))))
        })
        .times(1);

    // close channel to exit the infinite loop so we can
    // assert on resulting state
    mock_receiver
        .expect_recv()
        .returning(|| Err(eyre!("channel closed")))
        .times(1);

    let test = setup(mock_sender, mock_receiver);
    test.store.dispatch(Action::SetUIPaused(true));

    let _ = test.renderer.start_loop();

    let state = test.store.get_state();
    assert_eq!(state.error, Some("test error".to_string()));
}

#[test]
fn paused_loop_ignores_pause_message() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<RendererMessage>::new();

    mock_receiver
        .expect_recv()
        .returning(|| Ok(RendererMessage::PauseUI))
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Err(eyre!("channel closed")))
        .times(1);

    let test = setup(mock_sender, mock_receiver);
    test.store.dispatch(Action::SetUIPaused(true));

    let _ = test.renderer.start_loop();

    let state = test.store.get_state();
    assert!(state.ui_paused);
}

#[test]
fn paused_loop_exits_on_channel_close() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<RendererMessage>::new();

    mock_receiver
        .expect_recv()
        .returning(|| Err(eyre!("channel closed")))
        .times(1);

    let test = setup(mock_sender, mock_receiver);
    test.store.dispatch(Action::SetUIPaused(true));

    let result = test.renderer.start_loop();
    assert!(result.is_ok());
}

#[test]
fn paused_loop_processes_multiple_action_syncs() {
    let mock_sender = MockIpcSender::<MainMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<RendererMessage>::new();

    mock_receiver
        .expect_recv()
        .returning(|| {
            Ok(RendererMessage::ActionSync(Box::new(
                Action::UpdateMessage(Some("first".into())),
            )))
        })
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| {
            Ok(RendererMessage::ActionSync(Box::new(Action::SetError(
                Some("an error".into()),
            ))))
        })
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Err(eyre!("channel closed")))
        .times(1);

    let test = setup(mock_sender, mock_receiver);
    test.store.dispatch(Action::SetUIPaused(true));

    let _ = test.renderer.start_loop();

    let state = test.store.get_state();
    assert_eq!(state.message, Some("first".to_string()));
    assert_eq!(state.error, Some("an error".to_string()));
}

#[test]
fn paused_loop_resume_dispatches_unpause() {
    let mut mock_sender = MockIpcSender::<MainMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<RendererMessage>::new();

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, MainMessage::UIResumed))
        .returning(|_| Ok(()))
        .times(1);

    mock_sender.expect_box_clone().returning(|| {
        let mut s = MockIpcSender::<MainMessage>::new();
        s.expect_send().returning(|_| Ok(()));
        s.expect_box_clone()
            .returning(|| Box::new(MockIpcSender::<MainMessage>::new()));
        Box::new(s)
    });

    mock_receiver
        .expect_recv()
        .returning(|| Ok(RendererMessage::ResumeUI))
        .times(1);

    // after resume the unpaused branch calls try_recv;
    // re-pause via ActionSync so we can exit cleanly
    mock_receiver
        .expect_try_recv()
        .returning(|| {
            Ok(RendererMessage::ActionSync(Box::new(Action::SetUIPaused(
                true,
            ))))
        })
        .times(1);

    mock_receiver
        .expect_recv()
        .returning(|| Err(eyre!("channel closed")))
        .times(1);

    let test = setup(mock_sender, mock_receiver);
    test.store.dispatch(Action::SetUIPaused(true));

    let _ = test.renderer.start_loop();

    // ResumeUI dispatched SetUIPaused(false), then the
    // ActionSync re-paused it
    let state = test.store.get_state();
    assert!(state.ui_paused);
}

#[test]
fn pause_sends_ui_paused_message() {
    let mut mock_sender = MockIpcSender::<MainMessage>::new();
    let mut mock_receiver = MockIpcReceiver::<RendererMessage>::new();

    mock_sender
        .expect_send()
        .withf(|msg| matches!(msg, MainMessage::UIPaused))
        .returning(|_| Ok(()))
        .times(1);

    // unpaused branch: try_recv returns PauseUI
    mock_receiver
        .expect_try_recv()
        .returning(|| Ok(RendererMessage::PauseUI))
        .times(1);

    // now paused, channel close exits
    mock_receiver
        .expect_recv()
        .returning(|| Err(eyre!("channel closed")))
        .times(1);

    let test = setup(mock_sender, mock_receiver);

    let _ = test.renderer.start_loop();

    let state = test.store.get_state();
    assert!(state.ui_paused);
}
