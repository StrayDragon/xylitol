use std::sync::Arc;

use rmcp::model::{ServerNotification, SubscriptionFilter};

use super::client::{MCP_PROBE_TIMEOUT, McpClientManager, classify_probe_error};

pub(crate) fn resource_subscription_filter(uris: &[String]) -> SubscriptionFilter {
    let builder = SubscriptionFilter::builder().resources_list_changed();
    if uris.is_empty() {
        return builder.build();
    }
    builder.resource_subscriptions(uris.iter().cloned()).build()
}

pub(crate) async fn run_resource_subscription(
    manager: Arc<McpClientManager>,
    server: String,
    peer: rmcp::Peer<rmcp::RoleClient>,
) {
    let uris: Vec<String> =
        match tokio::time::timeout(MCP_PROBE_TIMEOUT, peer.list_all_resources()).await {
            Ok(Ok(resources)) => resources.into_iter().map(|resource| resource.uri).collect(),
            Ok(Err(error)) => {
                log::warn!("MCP subscription list resources failed server={server} error={error}");
                Vec::new()
            }
            Err(_) => {
                log::warn!("MCP subscription list resources timed out server={server}");
                Vec::new()
            }
        };
    let filter = resource_subscription_filter(&uris);
    let mut subscription = match tokio::time::timeout(MCP_PROBE_TIMEOUT, peer.listen(filter)).await
    {
        Ok(Ok(subscription)) => subscription,
        Ok(Err(error)) => {
            let message = error.to_string();
            if classify_probe_error(&message) == "unsupported" {
                log::warn!(
                    "MCP server {server}: subscriptions/listen unsupported; falling back to manual refresh"
                );
            } else {
                log::warn!("MCP server {server}: subscriptions/listen failed: {message}");
            }
            manager.mark_tools_dirty();
            return;
        }
        Err(_) => {
            log::warn!(
                "MCP server {server}: subscriptions/listen timed out; falling back to manual refresh"
            );
            manager.mark_tools_dirty();
            return;
        }
    };
    let acknowledged = subscription.acknowledged().clone();
    let list_changed = acknowledged.resources_list_changed == Some(true);
    let subscribed_uris = acknowledged
        .resource_subscriptions
        .as_deref()
        .is_some_and(|uris| !uris.is_empty());
    if !list_changed && !subscribed_uris {
        log::warn!(
            "MCP server {server}: subscription acknowledged no resource notifications; falling back to manual refresh"
        );
        let _ = subscription.cancel().await;
        manager.mark_tools_dirty();
        return;
    }
    loop {
        match subscription.next().await {
            Ok(Some(ServerNotification::ResourceUpdatedNotification(update))) => {
                manager
                    .invalidate_resource(&server, &update.params.uri)
                    .await;
            }
            Ok(Some(ServerNotification::ResourceListChangedNotification(_))) => {
                manager.invalidate_server_resources(&server).await;
                manager.mark_tools_dirty();
            }
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(error) => {
                log::warn!("MCP subscription ended with error server={server} error={error}");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_sets_resources_list_changed_and_uris() {
        let filter =
            resource_subscription_filter(&["memo://a".to_string(), "memo://b".to_string()]);
        assert_eq!(filter.resources_list_changed, Some(true));
        assert_eq!(
            filter.resource_subscriptions.as_deref(),
            Some(&["memo://a".to_string(), "memo://b".to_string()][..])
        );
        assert_eq!(filter.tools_list_changed, None);
        assert_eq!(filter.prompts_list_changed, None);
    }

    #[test]
    fn filter_omits_resource_subscriptions_when_no_uris() {
        let filter = resource_subscription_filter(&[]);
        assert_eq!(filter.resources_list_changed, Some(true));
        assert_eq!(filter.resource_subscriptions, None);
    }

    #[test]
    fn filter_is_subset_of_requested_scope() {
        let uris = vec!["memo://a".to_string()];
        let filter = resource_subscription_filter(&uris);
        assert!(filter.is_subset_of(&resource_subscription_filter(&uris)));
    }
}
