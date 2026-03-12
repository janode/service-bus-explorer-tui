pub fn send_target(entity_path: &str) -> &str {
    subscription_separator(entity_path)
        .map(|(idx, _)| &entity_path[..idx])
        .unwrap_or(entity_path)
}

pub fn split_subscription_path(entity_path: &str) -> Option<(&str, &str)> {
    let (idx, sep_len) = subscription_separator(entity_path)?;
    let topic = &entity_path[..idx];
    let subscription = &entity_path[idx + sep_len..];

    if topic.is_empty() || subscription.is_empty() {
        None
    } else {
        Some((topic, subscription))
    }
}

pub fn to_data_plane_path(entity_path: &str) -> String {
    entity_path.replace("/Subscriptions/", "/subscriptions/")
}

fn subscription_separator(entity_path: &str) -> Option<(usize, usize)> {
    entity_path
        .find("/Subscriptions/")
        .map(|idx| (idx, "/Subscriptions/".len()))
        .or_else(|| {
            entity_path
                .find("/subscriptions/")
                .map(|idx| (idx, "/subscriptions/".len()))
        })
}

#[cfg(test)]
mod tests {
    use super::{send_target, split_subscription_path, to_data_plane_path};

    #[test]
    fn send_target_returns_queue_or_topic_path() {
        assert_eq!(send_target("queue-a"), "queue-a");
        assert_eq!(send_target("topic-a"), "topic-a");
        assert_eq!(send_target("topic-a/Subscriptions/sub-a"), "topic-a");
        assert_eq!(send_target("topic-a/subscriptions/sub-a"), "topic-a");
    }

    #[test]
    fn split_subscription_path_handles_both_subscription_casings() {
        assert_eq!(
            split_subscription_path("topic-a/Subscriptions/sub-a"),
            Some(("topic-a", "sub-a"))
        );
        assert_eq!(
            split_subscription_path("topic-a/subscriptions/sub-a"),
            Some(("topic-a", "sub-a"))
        );
    }

    #[test]
    fn split_subscription_path_rejects_invalid_shapes() {
        assert_eq!(split_subscription_path("topic-a/Subscriptions/"), None);
        assert_eq!(split_subscription_path("/Subscriptions/sub-a"), None);
        assert_eq!(split_subscription_path("queue-a"), None);
    }

    #[test]
    fn to_data_plane_path_normalizes_subscription_segment() {
        assert_eq!(
            to_data_plane_path("topic-a/Subscriptions/sub-a"),
            "topic-a/subscriptions/sub-a"
        );
        assert_eq!(
            to_data_plane_path("topic-a/Subscriptions/sub-a/$deadletterqueue"),
            "topic-a/subscriptions/sub-a/$deadletterqueue"
        );
        assert_eq!(to_data_plane_path("queue-a"), "queue-a");
    }
}
