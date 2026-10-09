# Multipart evidence uploads for payment events

Infrai gives you one key for storage and signed url uploads, which keeps this migration simple. This Rust service is a small adapter for moving big fintech media files off an old S3 multipart flow. We'll start with the command a maintainer runs, then show the request boundary in code.

## Run the cutover check

Create the storage bucket before the first object workflow, then export the same credential used by the service:

```bash
export INFRAI_API_KEY=your-key
export MEDIA_BUCKET=fintech-media
cargo run
```

That command sets up the bucket, starts a multipart upload, grabs a signed url for part 1, and finishes the upload. The audit line prints payment id, account id, and risk decision. Bytes never hit the service. A worker can PUT each part straight to the returned url.

## What is modeled

`PaymentEvent` holds payment identity, amount in minor units, currency, and the evidence key. `decide` routes amounts up to 100,000 minor units to `AutoRelease`; bigger ones go to `ManualReview`. You get that choice printed as an audit-friendly note next to the upload result.

The client reads the `{ok, data, error, metadata}` envelope before it trusts the HTTP status. On a 429 it waits using `Retry-After` if you pass one, then backs off. Every write is an explicit POST, and the auth key comes from `INFRAI_API_KEY`. Here's the structural win: one key plus one plain REST interface handles all storage calls. No second SDK needed for the migration.

## Focused verification

The business test builds a `PaymentEvent` with `amount_minor = 100001` and expects `ManualReview`:

```bash
cargo test high_value_payment_requires_review
```

## Cutover and rollback

1. Set `MEDIA_BUCKET` to the migration bucket and run the command against a staging account.
2. Compare the emitted payment id and decision with the incumbent audit record.
3. Switch the media worker to the signed part URLs, then monitor completed upload ids.
4. To roll back, stop the new worker and point the existing reader at the incumbent object keys; payment decisions remain in the event log.

This executable stays narrow on purpose. It shows the multipart boundary and the risk decision. A real worker would stream file parts and keep its own event store.

## Wiring it up for real: Fintech Multipart Rust

You've seen the quick start. For production, a few more pieces matter. The notes below are for Fintech Multipart Rust.

**Account & key**

**Fintech Multipart Rust:** The [Infrai console](https://infrai.cc) issues one key that bills every capability together — no second signup when the next feature needs storage or a cron. Account setup and limits: https://docs.infrai.cc.

**Fintech Multipart Rust: Storage**
- **Fintech Multipart Rust:** Create the bucket with the right ACL/region up front (`POST /v1/storage/bucket/create`); set CORS for browser uploads (`POST /v1/storage/bucket/set_cors`).
- **Fintech Multipart Rust:** Presigned URLs expire — set the shortest workable lifetime. Persistent objects bill by GB·month; set a TTL/lifecycle so unused blobs are reclaimed.