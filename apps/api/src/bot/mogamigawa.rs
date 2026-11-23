use super::Bot;
use crate::traq::Message;
use anyhow::{Context, Result};

const STAMP_NAMES: [&str; 3] = ["mogamigawa01_mo", "mogamigawa02_gami", "mogamigawa03_gawa"];

impl Bot {
    pub(super) async fn complete_mogamigawa(&self, message: &Message) -> Result<()> {
        let message_id = message.id;
        let stamps = self
            .client
            .stamps()
            .await
            .with_context(|| format!("Could not load stamps for Mogamigawa on {message_id}"))?;
        let mut ordered_stamps = Vec::with_capacity(STAMP_NAMES.len());
        for name in STAMP_NAMES {
            let Some(stamp) = stamps.iter().find(|stamp| stamp.name == name) else {
                tracing::warn!(%message_id, stamp = name, "Mogamigawa stamp is missing");
                return Ok(());
            };
            ordered_stamps.push(stamp);
        }

        if !self.store.record_mogamigawa(message_id).await? {
            return Ok(());
        }

        let link = self.client.message_link(message_id);
        self.client
            .reply(message.channel_id, &format!("最上川\n{link}"))
            .await
            .with_context(|| format!("Could not post Mogamigawa reply for {message_id}"))?;

        for stamp in ordered_stamps {
            self.client
                .add_stamp(message_id, stamp.id, 1)
                .await
                .with_context(|| format!("Could not add stamp {} to {message_id}", stamp.name))?;
        }
        Ok(())
    }
}
