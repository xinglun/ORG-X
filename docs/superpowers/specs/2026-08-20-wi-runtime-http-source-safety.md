# Runtime Source URL and Redirect Safety

## Goal

Close the deterministic network-safety gap at the Weekly Radar source configuration and HTTP transport boundaries without inventing a provider allowlist or changing research behavior.

## Evidence-backed gap

`CompanyConfig::validate` currently accepts any string beginning with `http://` or `https://`, including URLs with credentials, fragments, loopback/private IP literals, and local-only host names. `ureq` follows redirects by default, so a valid configured URL can redirect the runtime to an unintended destination. Telegram uses a token-bearing URL and must also fail closed on redirects.

## Design

1. Parse configured source URLs with the `url` crate.
2. Require `http` or `https`, a non-empty host, no username/password, and no fragment.
3. Reject obvious local-only host names (`localhost`, `.localhost`, `.local`, `.internal`, `.lan`, and `.home.arpa`) and IP literals that are loopback, private, link-local, unspecified, multicast, or IPv4-mapped local addresses.
4. Keep reserved test domains such as `example.test` valid for injected fixture clients; this policy applies to configured runtime source URLs and does not resolve DNS names.
5. Configure both the reusable source `UreqHttpClient` and the Telegram transport agent with `redirects(0)`. A 3xx response is therefore returned as an ordinary non-success response and is classified as unavailable/rejected by the existing adapter boundary.
6. Keep error messages secret-safe: validation errors identify only the configuration field, never the full URL, credentials, query, or fragment.

## Non-goals

- No DNS resolution, DNS-rebinding defense, complete public-domain allowlist, proxy policy, or network egress firewall.
- No retry/backoff redesign.
- No changes to Stage, score, ranking, universe membership, validation horizons, Telegram payloads, or production workflow execution.

## Acceptance

- Invalid configured URLs are rejected before source collection.
- Existing configured public URLs and fixture URLs remain accepted.
- Credentials, fragments, malformed URLs, local-only names, and private/loopback/link-local IP literals are rejected without secret-bearing diagnostics.
- Source and Telegram HTTP agents do not follow redirects.
- Existing response-size, status, timeout, source parsing, and publication behavior remains unchanged.
- Focused security regression tests and the complete repository quality suite pass.
