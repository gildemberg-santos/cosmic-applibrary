use cosmic::iced::subscription;
use futures::{
    channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    StreamExt,
};
use std::{fmt::Debug, hash::Hash};
use zbus::{dbus_interface, Connection, ConnectionBuilder};

pub fn dbus_toggle<I: 'static + Hash + Copy + Send + Sync + Debug>(
    id: I,
) -> cosmic::iced::Subscription<(I, DbusEvent)> {
    subscription::unfold(id, State::Ready, move |state| start_listening(id, state))
}

#[derive(Debug)]
pub enum State {
    Ready,
    Waiting(Connection, UnboundedReceiver<DbusEvent>),
    Finished,
}

async fn start_listening<I: Copy>(id: I, state: State) -> (Option<(I, DbusEvent)>, State) {
    match state {
        State::Ready => {
            let (tx, rx) = unbounded();
            if let Some(conn) = ConnectionBuilder::session()
                .ok()
                .and_then(|conn| conn.name("com.system76.CosmicAppLibrary").ok())
                .and_then(|conn| {
                    conn.serve_at(
                        "/com/system76/CosmicAppLibrary",
                        CosmicAppLibraryServer { tx },
                    )
                    .ok()
                })
                .map(|conn| conn.build())
            {
                if let Ok(conn) = conn.await {
                    return (None, State::Waiting(conn, rx));
                }
            }
            return (None, State::Finished);
        }
        State::Waiting(conn, mut rx) => {
            if let Some(DbusEvent::Toggle) = rx.next().await {
                (Some((id, DbusEvent::Toggle)), State::Waiting(conn, rx))
            } else {
                (None, State::Finished)
            }
        }
        State::Finished => cosmic::iced::futures::future::pending().await,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DbusEvent {
    Toggle,
}

#[derive(Debug)]
pub(crate) struct CosmicAppLibraryServer {
    pub(crate) tx: UnboundedSender<DbusEvent>,
}

#[dbus_interface(name = "com.system76.CosmicAppLibrary")]
impl CosmicAppLibraryServer {
    async fn toggle(&self) {
        self.tx.unbounded_send(DbusEvent::Toggle).unwrap();
    }
}
