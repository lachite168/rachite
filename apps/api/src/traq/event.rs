use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum Event {
    MessageCreated {
        body: MessageBody<Message>,
    },
    DirectMessageCreated {
        body: MessageBody<Message>,
    },
    MessageUpdated {
        body: MessageBody<Message>,
    },
    MessageDeleted {
        body: MessageBody<DeletedMessage>,
    },
    Joined,
    Left,
    Ping,
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
pub(crate) struct MessageBody<T> {
    pub message: T,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Message {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub user: User,
    #[serde(default)]
    pub text: String,
    pub plain_text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub(crate) struct DeletedMessage {
    pub id: Uuid,
}

#[derive(Deserialize)]
pub(crate) struct User {
    pub id: Uuid,
    pub name: String,
    pub bot: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decodes_message_lifecycle_events_from_traq() {
        let id = Uuid::now_v7();
        let channel = Uuid::now_v7();
        let user = Uuid::now_v7();
        let message = json!({
            "id": id,
            "channelId": channel,
            "user": {"id": user, "name": "alice", "bot": false},
            "text": "new text",
            "plainText": "new text",
            "createdAt": "2026-11-23T01:02:03Z"
        });

        for event_type in [
            "MESSAGE_CREATED",
            "MESSAGE_UPDATED",
            "DIRECT_MESSAGE_CREATED",
        ] {
            let event: Event = serde_json::from_value(json!({
                "type": event_type,
                "reqId": "sample-request",
                "body": {"message": message}
            }))
            .unwrap();
            let message = match (event_type, event) {
                (
                    "MESSAGE_CREATED",
                    Event::MessageCreated {
                        body: MessageBody { message },
                    },
                )
                | (
                    "MESSAGE_UPDATED",
                    Event::MessageUpdated {
                        body: MessageBody { message },
                    },
                )
                | (
                    "DIRECT_MESSAGE_CREATED",
                    Event::DirectMessageCreated {
                        body: MessageBody { message },
                    },
                ) => message,
                _ => panic!("incorrectly decoded {event_type}"),
            };
            assert_eq!(message.id, id);
            assert_eq!(message.channel_id, channel);
            assert_eq!(message.user.name, "alice");
            assert_eq!(message.user.id, user);
            assert!(!message.user.bot);
            assert_eq!(message.plain_text, "new text");
        }

        let event: Event = serde_json::from_value(json!({
            "type": "MESSAGE_DELETED",
            "body": {"message": {"id": id, "channelId": channel}}
        }))
        .unwrap();
        assert!(
            matches!(event, Event::MessageDeleted { body: MessageBody { message } } if message.id == id)
        );
    }

    #[test]
    fn recognizes_ping_and_tolerates_unhandled_events() {
        let ping =
            r#"{"type":"PING","reqId":"request","body":{"eventTime":"2026-11-23T00:00:00Z"}}"#;
        let unknown = r#"{"type":"UNHANDLED_EVENT","body":"unhandled payload"}"#;
        assert!(matches!(serde_json::from_str(ping), Ok(Event::Ping)));
        assert!(matches!(serde_json::from_str(unknown), Ok(Event::Other)));
    }
}
