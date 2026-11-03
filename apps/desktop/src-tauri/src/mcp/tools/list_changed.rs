//! `notifications/tools/list_changed`, sent when a person turns a tool group
//! on or off in Settings > Agents. Sessions of the protocol versions before
//! 2026-07-28 get it on their stream from the moment they are initialized;
//! later ones ask for it with `subscriptions/listen`.

use rmcp::RoleServer;
use rmcp::model::SubscriptionFilter;
use rmcp::service::{NotificationContext, Peer, SubscriptionContext};
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::error::RecvError;

/// Waits for the next change; false once the hub is gone.
async fn next_change(changes: &mut Receiver<()>) -> bool {
    loop {
        match changes.recv().await {
            Ok(()) => return true,
            // Several changes at once still need only one notice.
            Err(RecvError::Lagged(_)) => {}
            Err(RecvError::Closed) => return false,
        }
    }
}

/// Forwards each change to a legacy session until its client goes away.
pub(super) fn forward_to_session(context: &NotificationContext<RoleServer>, changes: Receiver<()>) {
    let peer: Peer<RoleServer> = context.peer.clone();
    tokio::spawn(async move {
        let mut changes = changes;
        while next_change(&mut changes).await {
            if peer.is_transport_closed() || peer.notify_tool_list_changed().await.is_err() {
                break;
            }
        }
    });
}

/// The notifications a `subscriptions/listen` gets here: the tool list's.
pub(super) fn accepted(requested: &SubscriptionFilter) -> Option<SubscriptionFilter> {
    let mut accepted = SubscriptionFilter::default();
    accepted.tools_list_changed = requested.tools_list_changed.filter(|on| *on);
    Some(accepted)
}

/// Forwards each change to a subscription until it is cancelled.
pub(super) async fn listen(subscription: SubscriptionContext, mut changes: Receiver<()>) {
    loop {
        tokio::select! {
            () = subscription.cancelled() => return,
            changed = next_change(&mut changes) => {
                if !changed || subscription.sink().notify_tool_list_changed().await.is_err() {
                    return;
                }
            }
        }
    }
}
