# Contributing a Provider to AI Ranger

Adding a new provider to the AI Ranger registry is the simplest way to
contribute. It requires no code changes -- just a single edit to a TOML file.

AI Ranger detects AI provider traffic by matching TLS SNI hostnames against the
entries in `providers/providers.toml`. When you add a provider, every AI Ranger
deployment in the world will start detecting it on the next update.

---

## Provider Format

Each provider is a `[[providers]]` entry in `providers/providers.toml`. Here is
the complete set of fields:

| Field          | Type              | Required | Description |
|----------------|-------------------|----------|-------------|
| `name`         | string            | Yes      | Lowercase identifier used in events and internal matching. Use underscores for multi-word names (e.g. `"together"`, `"amazon_bedrock"`). |
| `display_name` | string            | Yes      | Human-readable name shown in the dashboard and reports (e.g. `"Together AI"`, `"Amazon Bedrock"`). |
| `hostnames`    | array of strings  | Yes      | Exact hostnames the provider uses for API and web traffic. Subdomains are matched automatically -- see note below. |
| `ip_ranges`    | array of strings  | No       | CIDR blocks for providers with dedicated IP space. See the [IP Ranges](#ip-ranges) section. |
| `docs_url`     | string            | No       | Link to the provider's API documentation. Helps reviewers verify hostnames. |

**Subdomain matching:** Hostnames are matched exactly and as parent domains.
If you add `"api.example.com"`, then traffic to `eu.api.example.com` will also
match. You do not need to list every regional subdomain.

### Complete Example

Suppose you are adding a fictional provider called "NovaMind" whose API lives at
`api.novamind.ai` and whose playground is at `app.novamind.ai`:

```toml
[[providers]]
name = "novamind"
display_name = "NovaMind"
hostnames = ["api.novamind.ai", "app.novamind.ai"]
docs_url = "https://docs.novamind.ai"
```

Append this block to the end of `providers/providers.toml` (before the Ollama
entry, which is a special case and stays last by convention).

---

## Finding the Correct Hostnames

Getting the hostnames right is the most important part. Here are four reliable
methods:

### 1. Check the provider's API documentation

Most providers document their base URL prominently. For example, if the docs
say "Send requests to `https://api.example.com/v1/chat`", the hostname is
`api.example.com`.

### 2. Browser DevTools

1. Open the provider's web interface (chat playground, dashboard, etc.).
2. Open DevTools (F12) and go to the **Network** tab.
3. Filter requests by the provider's domain.
4. Note the hostnames that appear in API calls (look for `/v1/`, `/api/`,
   `/chat/completions`, etc.).

### 3. Run AI Ranger in standalone mode

Start the agent without a backend configured. It will print every detected TLS
connection to stdout:

```bash
sudo ./target/debug/ai-ranger
```

Then, in another terminal, trigger traffic to the provider (e.g. use their CLI
tool, SDK, or `curl`). The agent will show the SNI hostname for each connection.
If the hostname is not yet in the registry, the event will have
`provider: null` -- that hostname is exactly what you need to add.

### 4. Verify with DNS

Use `nslookup` or `dig` to confirm the hostname resolves:

```bash
dig api.novamind.ai +short
```

If it resolves to an IP address, the hostname is real. If it returns a CNAME to
a CDN (e.g. `d1234.cloudfront.net`), the hostname is still correct to add --
AI Ranger matches on SNI, not on the resolved IP.

### What NOT to add

Do **not** add CDN hostnames that are shared by millions of sites:

- `*.cloudflare.com`
- `*.cloudfront.net`
- `*.akamaized.net`
- `*.fastly.net`
- `*.azureedge.net`

These will cause massive false positives. Always add the provider's own
hostname (e.g. `api.novamind.ai`), not the CDN hostname it resolves to.

---

## Testing Your Addition

After editing `providers/providers.toml`, verify it works end to end:

1. **Parse check** -- Run the Rust test suite to make sure your TOML is valid:

   ```bash
   cargo test
   ```

   If you introduced a syntax error or used an unrecognized field, this will
   catch it.

2. **Build the agent:**

   ```bash
   cargo build
   ```

3. **Start the agent** (requires root for raw socket access):

   ```bash
   sudo ./target/debug/ai-ranger
   ```

4. **Generate traffic** in another terminal:

   ```bash
   curl -s https://api.novamind.ai > /dev/null
   ```

5. **Check the output.** The agent should print a JSON event containing:

   ```json
   {
     "provider": "novamind",
     "hostname": "api.novamind.ai",
     ...
   }
   ```

   If `provider` shows the correct name, your entry is working.

---

## IP Ranges

IP ranges are an optional fallback detection mechanism. They are used only when
both SNI and DNS detection fail (e.g. when a browser uses Encrypted Client Hello
and DNS over HTTPS simultaneously).

### When to add IP ranges

Only add `ip_ranges` when **all** of the following are true:

- The provider owns dedicated IP address space (check ARIN/RIPE whois).
- The IP ranges are **not** shared with other unrelated services.
- The provider is **not** behind a shared CDN (Cloudflare, CloudFront, Akamai).

Currently, only Anthropic has `ip_ranges` in the registry. Most providers use
shared cloud infrastructure or CDNs, so their IP ranges would cause false
positives.

### How to find IP ranges

1. Look up the provider's domain with `whois`:

   ```bash
   whois $(dig +short api.novamind.ai | head -1)
   ```

2. Check if the owning organization matches the provider. If the result says
   "Cloudflare" or "Amazon" or "Google Cloud", the IPs are shared -- do not
   add them.

3. Some providers publish their IP ranges in documentation or status pages.
   Anthropic, for example, documents their IP ranges publicly.

### Format

IP ranges use CIDR notation, supporting both IPv4 and IPv6:

```toml
ip_ranges = ["198.51.100.0/24", "2001:db8::/32"]
```

---

## Submitting the PR

### Title format

```
providers: add <provider name>
```

Examples:
- `providers: add Fireworks AI`
- `providers: add Groq`

### PR description

Include the following in your pull request description:

- **What the provider is** -- a one-sentence description of the AI service.
- **Hostnames being added** -- list each hostname and what it is used for
  (API endpoint, web playground, etc.).
- **How you verified them** -- which of the methods above you used, and
  confirmation that `cargo test` passes.
- **IP ranges** (if applicable) -- source for the CIDR blocks.

### Checklist

- [ ] Entry appended to `providers/providers.toml`
- [ ] `name` is lowercase with underscores (no spaces, no hyphens)
- [ ] `hostnames` contains only provider-owned hostnames (no CDN domains)
- [ ] `cargo test` passes
- [ ] Tested with the agent or verified hostnames via documentation/DNS

---

## Providers We Would Love to Have

The following providers are known to be missing from the registry. If you use
any of them, consider adding them:

- **Fireworks AI** -- fast inference platform for open-source models
- **Groq** -- LPU-based inference with extremely low latency
- **Cerebras** -- wafer-scale inference engine
- **Modal** -- serverless GPU compute for ML workloads
- **RunPod** -- GPU cloud for inference and training
- **Anyscale** -- managed Ray platform for distributed AI
- **Lambda Labs** -- GPU cloud and inference API
- **SambaNova** -- custom AI accelerator platform

For each of these, the typical contribution involves finding the API hostname
(usually `api.<provider>.com` or similar), verifying it resolves, and adding
the `[[providers]]` entry.

---

## Questions?

If you are unsure whether a hostname is correct or whether IP ranges are
appropriate for a provider, open an issue first. We are happy to help you get
it right before you put in the work of a full PR.
