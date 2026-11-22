//! A minimal public catalog projection, independent of launch authority and private paths.
use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use gamenode_plugin_sdk::{
    Error, HostClient, idempotency_key,
    prost_types::{Struct, Value, value::Kind},
    proto::{DataPublishRequest, DiscoveryRecord, MutationOutcome, StorageDurability},
};

pub const DATASET: &str = "dev.richmont.gamenode.dolphin-emu.catalog.v1";

fn text(value: String) -> Value {
    Value {
        kind: Some(Kind::StringValue(value)),
    }
}

/// Catalog identity deliberately collapses copies/revisions sharing a disc ID. It is not an
/// installation identity or a GameNode catalog ID. Unknown discs retain the scanner's opaque ID.
pub fn publication(record: &DiscoveryRecord) -> DataPublishRequest {
    let disc_id = record.fields.get("game_id").cloned().unwrap_or_default();
    let resource_id = if disc_id.is_empty() {
        record.id.clone()
    } else {
        disc_id.clone()
    };
    let platform = match record.fields.get("platform").map(String::as_str) {
        Some("gamecube") => "gamecube",
        Some("wii") => "wii",
        _ => "unknown",
    };
    let mut fields = std::collections::BTreeMap::new();
    if !disc_id.is_empty() {
        fields.insert("disc_id".to_owned(), text(disc_id.clone()));
    }
    let title = record
        .fields
        .get("title")
        .cloned()
        .unwrap_or_else(|| record.name.clone());
    for (key, value) in [
        ("title", title),
        ("platform", platform.to_owned()),
        ("content_id", resource_id.clone()),
    ] {
        fields.insert(key.to_owned(), text(value));
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    DataPublishRequest {
        dataset_id: DATASET.into(),
        schema_id: DATASET.into(),
        schema_version: 1,
        durability: StorageDurability::Persistent as i32,
        resource_id,
        value: Some(Struct { fields }),
        observed_at: now.as_secs().min(i64::MAX as u64) as i64,
        // Durable replacement is keyed by owner/dataset/resource, not this observation sequence.
        sequence: now.as_nanos().min(u64::MAX as u128) as u64,
        source_observation_id: idempotency_key(),
        ..Default::default()
    }
}

/// Failure is reported separately from discovery. Never downgrade persistence or log host text.
pub async fn publish(host: &HostClient, records: &[DiscoveryRecord]) -> Result<(), Error> {
    let resources = host.resources().ok_or(Error::Protocol)?;
    let mut seen = BTreeSet::new();
    for record in records {
        let request = publication(record);
        if !seen.insert(request.resource_id.clone()) {
            continue;
        }
        let reply = resources
            .data_publish(&idempotency_key(), 0, request)
            .await
            .map_err(|status| {
                eprintln!("dolphin-emu DataPublish: {:?}", status.code());
                Error::from(status)
            })?;
        if reply.durability != StorageDurability::Persistent as i32
            || !matches!(
                MutationOutcome::try_from(reply.outcome),
                Ok(MutationOutcome::Applied | MutationOutcome::AlreadyApplied)
            )
        {
            return Err(Error::Protocol);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn record(id: &str, game_id: &str, platform: &str) -> DiscoveryRecord {
        DiscoveryRecord {
            id: id.to_owned(),
            name: "Homebrew".to_owned(),
            install_dir: "/games".to_owned(),
            fields: HashMap::from([
                ("rom".to_owned(), format!("{id}.dol")),
                ("game_id".to_owned(), game_id.to_owned()),
                ("platform".to_owned(), platform.to_owned()),
            ]),
            target: None,
        }
    }

    fn field<'a>(request: &'a DataPublishRequest, key: &str) -> Option<&'a str> {
        match request.value.as_ref()?.fields.get(key)?.kind.as_ref()? {
            Kind::StringValue(value) => Some(value),
            _ => None,
        }
    }

    #[test]
    fn headerless_records_omit_the_disc_id() {
        let request = publication(&record("dolphin-emu-0123456789abcdef", "", "unknown"));
        assert_eq!(request.resource_id, "dolphin-emu-0123456789abcdef");
        assert_eq!(
            field(&request, "content_id"),
            Some("dolphin-emu-0123456789abcdef")
        );
        assert_eq!(field(&request, "platform"), Some("unknown"));
        assert!(field(&request, "disc_id").is_none());
        assert_eq!(request.dataset_id, DATASET);
        assert_eq!(request.schema_version, 1);
    }

    #[test]
    fn discs_keep_their_disc_id() {
        let request = publication(&record("GZLE01", "GZLE01", "gamecube"));
        assert_eq!(request.resource_id, "GZLE01");
        assert_eq!(field(&request, "disc_id"), Some("GZLE01"));
        assert_eq!(field(&request, "platform"), Some("gamecube"));
    }

    #[test]
    fn the_published_title_prefers_the_disc_header() {
        let mut with_title = record("GPIE01", "GPIE01", "gamecube");
        with_title
            .fields
            .insert("title".to_owned(), "Pikmin".to_owned());
        assert_eq!(field(&publication(&with_title), "title"), Some("Pikmin"));

        let fallback = record("GZLE01", "GZLE01", "gamecube");
        assert_eq!(
            field(&publication(&fallback), "title"),
            Some("Homebrew"),
            "a record without a header title keeps the cleaned file name"
        );
    }
}
