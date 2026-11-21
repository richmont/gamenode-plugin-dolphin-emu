//! A minimal public catalog projection, independent of launch authority and private paths.
use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use gamenode_plugin_sdk::{
    Error, HostClient, idempotency_key,
    prost_types::{Struct, Value, value::Kind},
    proto::{DataPublishRequest, DiscoveryRecord, MutationOutcome, StorageDurability},
};

pub const DATASET: &str = "org.gamenode.dolphin.catalog.v1";

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
    let fields = [
        ("disc_id", disc_id),
        ("title", record.name.clone()),
        ("platform", platform.into()),
        ("content_id", resource_id.clone()),
    ]
    .into_iter()
    .map(|(key, value)| {
        (
            key.into(),
            Value {
                kind: Some(Kind::StringValue(value)),
            },
        )
    })
    .collect();
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
                eprintln!("dolphin DataPublish: {:?}", status.code());
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
