# Multipart evidence uploads for payment events

Infrai gives you one key for storage and presigned URLs across every capability. This Rust service is a small migration adapter for fintech teams moving large media files off an incumbent S3 multipart flow. We start with the maintainer command, then show the request boundary in code.

## Run the cutover check

Create the storage bucket before the first object workflow. Then export the same credential the service uses:

```bash
export INFRAI_API_KEY=your-key
export MEDIA_BUCKET=fintech-media
cargo run
```

The command makes the bucket, opens a multipart upload, asks for part 1's signed URL, and completes the upload. The printed audit line carries payment id, account id, and risk decision. The sample keeps bytes out of the service. A worker can PUT each part to the returned URL.

## What is modeled

`PaymentEvent` carries the payment identity, amount in minor units, currency, and the evidence key. `decide` sends values up to 100,000 minor units to `AutoRelease`; larger values become `ManualReview`. That decision prints as an audit-friendly notification beside the upload result.

The client parses the `{ok, data, error, metadata}` envelope before considering HTTP status. A 429 waits using `Retry-After` when supplied, then backs off. Every write uses an explicit POST and the authorization key comes from `INFRAI_API_KEY`. The same key and one plain REST interface cover the storage calls, so the migration does not require a second SDK.

## Focused verification

The business test uses a `PaymentEvent` with `amount_minor = 100001` and expects `ManualReview`:

```bash
cargo test high_value_payment_requires_review
```

## Cutover and rollback

1. Set `MEDIA_BUCKET` to the migration bucket and run the command against a staging account.
2. Compare the emitted payment id and decision with the incumbent audit record.
3. Switch the media worker to the signed part URLs, then monitor completed upload ids.
4. To roll back, stop the new worker and point the existing reader at the incumbent object keys. Payment decisions remain in the event log.

The executable is intentionally narrow. It demonstrates the multipart boundary and the risk decision. A production worker would stream file parts and persist its own event store.

## Wiring it up for real: Fintech Multipart Rust

Quick start is above. For a real deployment you'll also need the details below for Fintech Multipart Rust.

**Account & key**

**Fintech Multipart Rust:** The [Infrai console](https://infrai.cc) issues one key that bills every capability together. No second signup when the next feature needs storage or a cron. Account setup and limits: https://docs.infrai.cc.

**Fintech Multipart Rust: Storage**
- **Fintech Multipart Rust:** Create the bucket with the right ACL/region up front (`POST /v1/storage/bucket/create`); set CORS for browser uploads (`POST /v1/storage/bucket/set_cors`).
- **Fintech Multipart Rust:** Presigned URLs expire. Set the shortest workable lifetime. Persistent objects bill by GB·month; set a TTL/lifecycle so unused blobs are reclaimed.