//! State machine to manage the current mode of operation during peer lifecycle.

use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use librad::{
    net::{peer::Gossip, protocol::ProtocolEvent},
    peer::PeerId,
};

use crate::peer::announcement;

/// Default time to wait between announcement subroutine runs.
const DEFAULT_ANNOUNCE_INTERVAL: Duration = std::time::Duration::from_secs(60);

/// Default number of peers a full sync is attempting with up on startup.
/// TODO(xla): Revise number.
const DEFAULT_SYNC_MAX_PEERS: usize = 5;

/// Default Duration until the local peer goes online regardless if and how many syncs have
/// succeeded.
// TODO(xla): Review duration.
const DEFAULT_SYNC_PERIOD: Duration = Duration::from_secs(5);

/// Instructions to issue side-effectful operations which are the results from state transitions.
#[derive(Debug, PartialEq)]
pub enum Command {
    /// Start the announcement subroutine.
    Announce,
    /// Initiate a full sync with `PeerId`.
    SyncPeer(PeerId),
    /// Start sync timeout.
    StartSyncTimeout(Duration),
}

/// Significant events that occur during [`Peer`] lifetime.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug)]
pub enum Event {
    /// Announcement subroutine lifecycle events.
    Announce(AnnounceEvent),
    /// Events from the underlying coco protocol.
    Protocol(ProtocolEvent<Gossip>),
    /// Lifecycle events during peer sync operations.
    PeerSync(SyncEvent),
    /// Scheduled timeouts which can occur.
    Timeout(TimeoutEvent),
}

/// Announcement subroutine lifecycle events.
#[derive(Clone, Debug)]
pub enum AnnounceEvent {
    /// Operation failed.
    Failed,
    /// Operation succeeded and emitted the enclosed list of updates.
    Succeeded(announcement::Updates),
    /// The ticker duration has elapsed.
    Tick,
}

/// Lifecycle events during peer sync operations.
#[derive(Clone, Debug)]
pub enum SyncEvent {
    /// A sync has been initiated for `PeerId`.
    Started(PeerId),
    /// A sync has failed for `PeerId`.
    Failed(PeerId),
    /// A sync has succeeded for `PeerId`.
    Succeeded(PeerId),
}

/// Scheduled timeouts which can occur.
#[derive(Clone, Debug)]
pub enum TimeoutEvent {
    /// Grace period is over signaling that we should go offline, no matter how many syncs have
    /// succeeded.
    SyncPeriod,
}

/// The current status of the local peer and its relation to the network.
#[derive(Debug)]
enum Status {
    /// Nothing is setup, not even a socket to listen on.
    Stopped(Instant),
    /// Local peer is listening on a socket but has not connected to any peers yet.
    Started(Instant),
    /// The local peer lost its connections to all its peers.
    Offline(Instant),
    /// Phase where the local peer tries get up-to-date.
    Syncing(Instant, usize),
    /// The local peer is operational and is able to interact with the peers it has connected to.
    Online(Instant),
}

/// Set of knobs to change the behaviour of the [`RunState`].
#[derive(Default)]
pub struct Config {
    /// Set of knobs to alter announce behaviour.
    pub announce: AnnounceConfig,
    /// Set of knobs to alter sync behaviour.
    pub sync: SyncConfig,
}

/// Set of knobs to alter announce behaviour.
pub struct AnnounceConfig {
    /// Determines how often the announcement subroutine should be run.
    pub interval: Duration,
}

impl Default for AnnounceConfig {
    fn default() -> Self {
        Self {
            interval: DEFAULT_ANNOUNCE_INTERVAL,
        }
    }
}

/// Set of knobs to alter sync behaviour.
pub struct SyncConfig {
    /// Number of peers that a full sync is attempted with upon startup.
    pub max_peers: usize,
    /// Enables the syncing stage when coming online.
    pub on_startup: bool,
    /// Duration until the local peer goes online regardless if and how many syncs have succeeded.
    pub period: Duration,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_peers: DEFAULT_SYNC_MAX_PEERS,
            on_startup: false,
            period: DEFAULT_SYNC_PERIOD,
        }
    }
}

/// State kept for a running local peer.
pub struct RunState {
    /// Confiugration to change how input [`Event`]s are interpreted.
    config: Config,
    /// Tracking remote peers that have an active connection.
    connected_peers: HashSet<PeerId>,
    /// Current internal status.
    status: Status,
}

impl From<Config> for RunState {
    fn from(config: Config) -> Self {
        Self {
            config,
            connected_peers: HashSet::new(),
            status: Status::Stopped(Instant::now()),
        }
    }
}

impl RunState {
    /// Constructs a new state.
    #[cfg(test)]
    const fn new(config: Config, connected_peers: HashSet<PeerId>, status: Status) -> Self {
        Self {
            config,
            connected_peers,
            status,
        }
    }

    /// Applies the `event` and based on the current state transforms to the new state and in some
    /// cases produes commands which should be executed in the appropriate sub-routines.
    pub fn transition(&mut self, event: Event) -> Vec<Command> {
        log::trace!("run_state: ({:?}, {:?})", self.status, event);

        match (&self.status, event) {
            // Go from [`Status::Stopped`] to [`Status::Started`] once we are listening.
            (Status::Stopped(_since), Event::Protocol(ProtocolEvent::Listening(_addr))) => {
                self.status = Status::Started(Instant::now());

                vec![]
            },
            // Sync with first incoming peer.
            //
            // In case the peer is configured to sync on startup we start syncing, otherwise we go
            // online straight away.
            // TODO(xla): Also issue sync if we come online after a certain period of being
            // disconnected from any peer.
            (Status::Started(_since), Event::Protocol(ProtocolEvent::Connected(ref peer_id))) => {
                self.connected_peers.insert(peer_id.clone());

                if self.config.sync.on_startup {
                    self.status = Status::Syncing(Instant::now(), 1);

                    vec![
                        Command::SyncPeer(peer_id.clone()),
                        Command::StartSyncTimeout(self.config.sync.period),
                    ]
                } else {
                    self.status = Status::Online(Instant::now());

                    vec![]
                }
            },
            // Sync until configured maximum of peers is reached.
            (Status::Syncing(since, syncs), Event::Protocol(ProtocolEvent::Connected(peer_id)))
                if *syncs < self.config.sync.max_peers =>
            {
                self.connected_peers.insert(peer_id.clone());
                if syncs + 1 == self.config.sync.max_peers {
                    self.status = Status::Online(Instant::now());
                } else {
                    self.status = Status::Syncing(*since, syncs + 1);
                }

                vec![Command::SyncPeer(peer_id)]
            }
            // Go online if we exceed the sync period.
            (Status::Syncing(_since, _syncs), Event::Timeout(TimeoutEvent::SyncPeriod)) => {
                self.status = Status::Online(Instant::now());

                vec![]
            },
            // Remove peer that just disconnected.
            (_, Event::Protocol(ProtocolEvent::Disconnecting(peer_id))) => {
                self.connected_peers.remove(&peer_id);

                // Go offline if we have no more connected peers left.
                if self.connected_peers.is_empty() {
                    self.status = Status::Offline(Instant::now());
                }

                vec![]
            },
            // Announce new updates while the peer is online.
            (
                Status::Online(_) | Status::Started(_) | Status::Syncing(_, _),
                Event::Announce(AnnounceEvent::Tick),
            ) => vec![Command::Announce],
            _ => vec![],
        }
    }
}

#[allow(clippy::needless_update, clippy::panic, clippy::unwrap_used)]
#[cfg(test)]
mod test {
    use std::{
        collections::HashSet,
        iter::FromIterator,
        net::SocketAddr,
        time::{Duration, Instant},
    };

    use assert_matches::assert_matches;
    use pretty_assertions::assert_eq;

    use librad::{keys::SecretKey, net::protocol::ProtocolEvent, peer::PeerId};

    use super::{
        AnnounceEvent, Command, Config, Event, RunState, Status, SyncConfig, TimeoutEvent,
        DEFAULT_SYNC_MAX_PEERS,
    };

    #[test]
    fn transition_to_started_on_listen() -> Result<(), Box<dyn std::error::Error>> {
        let addr = "127.0.0.1:12345".parse::<SocketAddr>()?;

        let status = Status::Stopped(Instant::now());
        let mut state = RunState::new(Config::default(), HashSet::new(), status);

        let cmds = state.transition(Event::Protocol(ProtocolEvent::Listening(addr)));
        assert!(cmds.is_empty());
        assert_matches!(state.status, Status::Started(_));

        Ok(())
    }

    #[test]
    fn transition_to_online_if_sync_is_disabled() {
        let status = Status::Started(Instant::now());
        let mut state = RunState::new(
            Config {
                sync: SyncConfig {
                    on_startup: false,
                    ..SyncConfig::default()
                },
                ..Config::default()
            },
            HashSet::new(),
            status,
        );

        let cmds = {
            let key = SecretKey::new();
            let peer_id = PeerId::from(key);
            state.transition(Event::Protocol(ProtocolEvent::Connected(peer_id)))
        };
        assert!(cmds.is_empty());
        assert_matches!(state.status, Status::Online(_));
    }

    #[test]
    fn transition_to_online_after_sync_max_peers() {
        let status = Status::Syncing(Instant::now(), DEFAULT_SYNC_MAX_PEERS - 1);
        let mut state = RunState::new(Config::default(), HashSet::new(), status);

        let _cmds = {
            let key = SecretKey::new();
            let peer_id = PeerId::from(key);
            state.transition(Event::Protocol(ProtocolEvent::Connected(peer_id)))
        };
        assert_matches!(state.status, Status::Online(_));
    }

    #[test]
    fn transition_to_online_after_sync_period() {
        let status = Status::Syncing(Instant::now(), 3);
        let mut state = RunState::new(Config::default(), HashSet::new(), status);

        let _cmds = state.transition(Event::Timeout(TimeoutEvent::SyncPeriod));
        assert_matches!(state.status, Status::Online(_));
    }

    #[test]
    fn transition_to_offline_when_last_peer_disconnects() {
        let peer_id = PeerId::from(SecretKey::new());
        let status = Status::Online(Instant::now());
        let mut state = RunState::new(
            Config::default(),
            HashSet::from_iter(vec![peer_id.clone()]),
            status,
        );

        let _cmds = state.transition(Event::Protocol(ProtocolEvent::Disconnecting(peer_id)));
        assert_matches!(state.status, Status::Offline(_));
    }

    #[test]
    fn issue_sync_command_until_max_peers() {
        let max_peers = 13;
        let status = Status::Started(Instant::now());
        let mut state = RunState::new(
            Config {
                sync: SyncConfig {
                    max_peers,
                    on_startup: true,
                    ..SyncConfig::default()
                },
                ..Config::default()
            },
            HashSet::new(),
            status,
        );

        for i in 0..(max_peers - 1) {
            let key = SecretKey::new();
            let peer_id = PeerId::from(key);

            // Expect to sync with the first connected peer.
            let cmds = state.transition(Event::Protocol(ProtocolEvent::Connected(peer_id.clone())));
            assert!(!cmds.is_empty(), "expected command");
            assert_matches!(cmds.first().unwrap(), Command::SyncPeer(sync_id) => {
                assert_eq!(*sync_id, peer_id);
            });
            assert_matches!(state.status, Status::Syncing(_, syncing_peers) => {
                assert_eq!(i + 1, syncing_peers);
            });
        }

        // Issue last sync.
        let cmds = {
            let key = SecretKey::new();
            let peer_id = PeerId::from(key);
            state.transition(Event::Protocol(ProtocolEvent::Connected(peer_id)))
        };
        assert!(!cmds.is_empty(), "expected command");
        assert_matches!(cmds.first().unwrap(), Command::SyncPeer{..});
        // Expect to be online at this point.
        assert_matches!(state.status, Status::Online(_));

        // No more syncs should be expected after the maximum of peers have connected.
        let cmd = {
            let key = SecretKey::new();
            let peer_id = PeerId::from(key);
            state.transition(Event::Protocol(ProtocolEvent::Connected(peer_id)))
        };
        assert!(cmd.is_empty(), "should not emit any more commands");
    }

    #[test]
    fn issue_sync_timeout_when_transitioning_to_syncing() {
        let sync_period = Duration::from_secs(60 * 10);
        let status = Status::Started(Instant::now());
        let mut state = RunState::new(
            Config {
                sync: SyncConfig {
                    on_startup: true,
                    period: sync_period,
                    ..SyncConfig::default()
                },
                ..Config::default()
            },
            HashSet::new(),
            status,
        );

        let cmds = {
            let key = SecretKey::new();
            let peer_id = PeerId::from(key);
            state.transition(Event::Protocol(ProtocolEvent::Connected(peer_id)))
        };
        assert_matches!(cmds.get(1), Some(Command::StartSyncTimeout(period)) => {
            assert_eq!(*period, sync_period);
        });
    }

    #[test]
    fn issue_announce_while_online() {
        let status = Status::Online(Instant::now());
        let mut state = RunState::new(Config::default(), HashSet::new(), status);
        let cmds = state.transition(Event::Announce(AnnounceEvent::Tick));

        assert!(!cmds.is_empty(), "expected command");
        assert_matches!(cmds.first().unwrap(), Command::Announce);

        let status = Status::Offline(Instant::now());
        let mut state = RunState::new(Config::default(), HashSet::new(), status);
        let cmds = state.transition(Event::Announce(AnnounceEvent::Tick));

        assert!(cmds.is_empty(), "expected no command");
    }
}