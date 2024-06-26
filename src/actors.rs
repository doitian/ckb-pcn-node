use log::debug;
use ractor::{async_trait as rasync_trait, Actor, ActorProcessingErr, ActorRef};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

/// A root actor that listens for cancellation token and stops all sub actors (those who started by spawn_linked).
pub struct RootActor;

pub type RootActorMessage = String;

impl RootActor {
    pub async fn start(
        tracker: TaskTracker,
        token: CancellationToken,
    ) -> ActorRef<RootActorMessage> {
        Actor::spawn(
            Some("root actor".to_string()),
            RootActor {},
            (tracker, token),
        )
        .await
        .expect("start root actor")
        .0
    }
}

#[rasync_trait]
impl Actor for RootActor {
    type Msg = RootActorMessage;
    type State = ();
    type Arguments = (TaskTracker, CancellationToken);

    /// Spawn a thread that waits for token to be cancelled,
    /// after that kill all sub actors.
    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        (tracker, token): Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let cloned_myself = myself.clone();
        tracker.spawn(async move {
            token.cancelled().await;
            debug!("Shutting down root actor due to cancellation token");
            cloned_myself.stop(Some("Cancellation token received".to_owned()));
        });
        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        debug!("Root actor stopped");
        Ok(())
    }
}
