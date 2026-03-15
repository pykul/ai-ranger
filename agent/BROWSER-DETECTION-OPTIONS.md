# Browser AI Detection: Technical Options Document

## The Problem

Modern browsers deploy two privacy features that defeat AI Ranger's current detection:

- **ECH (Encrypted Client Hello)**: The real SNI hostname is encrypted; the outer ClientHello contains a dummy hostname (e.g. `cloudflare-ech.com`). Our SNI parser reads the dummy.
- **DoH (DNS over HTTPS)**: DNS queries go to `dns.google:443` or `cloudflare-dns.com:443` over HTTPS, bypassing UDP port 53 entirely. Our DNS capture never sees them.

Together, these eliminate both signals the agent relies on. CLI tools, SDKs, and desktop AI apps (Cursor, Claude Code, Copilot) do **not** use ECH and are unaffected. This is a browser-specific gap.

---

## Option 1: IP-Based Detection

**How it works**: Resolve AI provider hostnames to IP ranges. Match outbound TCP connections by destination IP instead of hostname.

**Current provider infrastructure**:

| Provider | Infrastructure | Dedicated IPs? |
|---|---|---|
| Anthropic API (`api.anthropic.com`) | Own ASN (AS399358), `160.79.104.0/23` | Yes |
| Claude.ai (web) | CloudFront + Cloudflare | No — shared CDN IPs |
| OpenAI (`api.openai.com`, `chat.openai.com`) | Cloudflare Anycast (`172.66.x.x`, `162.159.x.x`) | No — shared with millions of sites |
| Google Gemini | Google infrastructure | No — shared with all Google services |
| Cursor, Copilot | Cloudflare / Azure CDN | No |

**What it can detect**: Connections to providers with dedicated IP ranges (currently only Anthropic's API).

**What it cannot detect**: Any provider behind Cloudflare/CloudFront/Azure CDN. Matching on `172.66.x.x` would flag every Cloudflare customer on the internet. False positive rate is catastrophic.

**Complexity**: Low. IP set matching on observed connections.

**Platform support**: All.

**User-facing changes**: None.

**Tradeoffs**: Only viable as a supplementary signal for the few providers with dedicated ranges. Not a primary detection mechanism. IP ranges change over time and require periodic refresh.

---

## Option 2: OS-Level DNS Interception APIs

**How it works**: Instead of capturing DNS packets on the wire, tap into the OS's own DNS resolver to see hostname-to-IP mappings before they reach the network. This works regardless of whether the DNS query goes over UDP 53 or DoH, as long as the application uses the system resolver.

### Windows: ETW `Microsoft-Windows-DNS-Client`

**Status: Implemented.** The agent subscribes to this provider via `ferrisetw` on Windows. See `agent/src/capture/etw_dns.rs`.

The `Microsoft-Windows-DNS-Client` ETW provider (GUID `{1C95126E-7EEA-49A9-A3FE-A378B03DDB4D}`) emits events for every DNS resolution through the Windows DNS client service:

- **Event ID 3008**: Query completed — contains `QueryName` (hostname), `QueryResults` (resolved IPs), and `ProcessId`
- Already requires Administrator (agent already does)
- Implemented via `ferrisetw` crate (MIT/Apache-2.0)

**Limitation**: Browsers that use their own internal DoH resolver (Chrome, Firefox, Edge, Brave, and others) bypass the Windows DNS client entirely, so ETW DNS-Client events do not fire for those connections. Enterprise policy (`DnsOverHttpsMode`) can force browsers to use system DNS.

### Linux: eBPF uprobes on `getaddrinfo()`

**Status: Not yet implemented.**

Attach eBPF uprobes to `getaddrinfo()` / `gethostbyname()` in libc. Every DNS resolution at the application level is intercepted regardless of transport:

- Gets: PID, process name, hostname, resolved IP
- Requires root + kernel 4.5+ (already required for `AF_PACKET`)
- Rust implementation via `aya` crate
- Proven concept: BCC tool `gethostlatency` does exactly this

**Limitation**: If an application resolves DNS internally without calling libc (e.g. Chrome's internal DoH), the uprobe doesn't fire.

### macOS: mDNSResponder / NEDNSProxyProvider

**Status: Not yet implemented.**

Two approaches:

- **mDNSResponder log parsing**: Parse system DNS resolver logs. Pragmatic but fragile (log format not stable).
- **NEDNSProxyProvider**: Network Extension framework that intercepts all DNS system-wide. Powerful but requires a System Extension, Apple Developer notarization, and explicit user approval in System Preferences.

### Summary for Option 2

**What it can detect**: All DNS resolutions through system APIs — hostname, resolved IP, PID. Combined with connection tracking, gives full hostname-to-connection mapping regardless of ECH.

**What it cannot detect**: Applications that fully bypass system DNS with their own resolver (Chrome with built-in DoH not overridden by policy).

**Complexity**: Medium. Different implementation per platform. ETW on Windows is the most straightforward.

**Platform support**: All, but implementation differs significantly.

**User-facing changes**: None beyond existing admin/root requirement. Enterprise can force browsers to use system DNS via policy, closing the bypass gap entirely.

**Tradeoffs**: This is the strongest passive approach. The main gap (browser-internal DoH bypass) is closeable via enterprise policy. For unmanaged machines where the user controls Chrome settings, this gap remains.

---

## Option 3: Local DoH Resolver

**How it works**: Run a local DNS-over-HTTPS server on `127.0.0.1`. Configure the OS or browser to use it as the DoH upstream. The local resolver sees all queries in plaintext, logs hostname-to-IP mappings, and forwards upstream.

**Browser configuration**:

- Chrome: Enterprise policy `DnsOverHttpsTemplates` = `https://localhost:8053/dns-query` (requires admin for registry/GPO)
- Firefox: `policies.json` or `network.trr.uri` in `about:config`
- System-level: Set OS DNS to `127.0.0.1` where local resolver listens

**What it can detect**: Every DNS query routed through it — hostname, resolved IPs.

**What it cannot detect**: Applications that hardcode their own DoH provider and ignore system/browser DNS settings.

**Complexity**: Medium-high. Must build or bundle a DoH server, manage its lifecycle, handle failure modes (if local resolver crashes, all DNS breaks).

**Platform support**: All, but configuration differs per OS and browser.

**User-facing changes**: Requires changing DNS settings (admin-level). Invisible to users on managed machines via policy. On unmanaged machines, requires explicit consent.

**Tradeoffs**: Reliable when deployed, but introduces a critical dependency — if the local resolver fails, the machine loses DNS entirely. More invasive than OS-level interception (Option 2) and provides roughly the same information. Option 2 is generally preferable because it observes without inserting itself into the resolution path.

---

## Option 4: MITM Proxy (Phase 5, already planned)

**How it works**: Install a local CA certificate in the OS trust store. Run a TLS-terminating proxy for known AI provider hostnames only. Decrypt, inspect, re-encrypt.

**What it can detect**: Everything — hostname, full request/response, API keys, model names, token counts, prompt content.

**What it cannot detect**: Traffic from applications with hard-coded certificate pinning that reject the local CA (some Electron apps, mobile apps).

**Practical issues**:

- Browsers accept user-installed CAs — no resistance from Chrome/Firefox/Edge
- Firefox uses its own cert store (must install CA in both OS and Firefox)
- HPKP (HTTP Public Key Pinning) was deprecated by all browsers by 2019, replaced by Certificate Transparency
- Some non-browser apps may pin certs (Cursor's Electron shell, some Python SDKs)
- Performance overhead from TLS termination/re-encryption, especially for streaming responses
- PII exposure: decrypted prompts may contain sensitive data — requires explicit user consent

**Complexity**: High. CA generation, OS trust store integration per platform, transparent proxy, HTTP/2 handling, streaming support.

**Platform support**: All, but CA installation and proxy configuration differ.

**User-facing changes**: Must approve CA certificate installation. May need proxy configuration.

**Legal/ethical**: Decrypting traffic requires explicit notice in many jurisdictions. Organizations must have clear policies. This is fundamentally different from passive observation.

---

## Option 5: Browser Extension (comparison point)

Not proposed as an implementation option, but useful as a comparison baseline.

**What it can detect**: Full URL of every request (hostname, path, query params), request/response headers, tab context. Perfect hostname visibility regardless of ECH/DoH — operates above the TLS layer. Chrome MV3 `webRequest` API is read-only (observation, no blocking), sufficient for detection.

**What it cannot detect**: Non-browser traffic (CLI tools, SDKs, desktop apps).

**Tradeoffs vs network-level**: Browser extension has perfect visibility within its browser but zero visibility outside it. Network-level has universal visibility but struggles with ECH/DoH. They are complementary, not competing. Extension requires per-browser installation and the user can disable it.

---

## Option 6: Other Approaches

### Windows Filtering Platform (WFP)

Kernel-level network filtering API. Provides source/dest IP+port, protocol, and **process path** for every connection at the kernel level. More reliable process attribution than `GetExtendedTcpTable` (which is a point-in-time snapshot). However, WFP gives connection metadata only — no hostnames. Useful combined with ETW DNS events: ETW gives hostname-to-IP, WFP gives connection-to-process.

### Network Namespace Tricks (Linux)

Run applications in a network namespace with a controlled gateway. Requires containerization. Not viable for monitoring arbitrary existing processes.

---

## Summary Matrix

| Approach | Detects browser AI? | False positive risk | Complexity | User changes | Invasiveness |
|---|---|---|---|---|---|
| IP-based | Partial (dedicated IPs only) | High for CDN providers | Low | None | Passive |
| OS-level DNS (ETW/eBPF) | Yes (unless browser bypasses system DNS) | Low | Medium | None | Passive |
| Local DoH resolver | Yes (if browser configured to use it) | Low | Medium-high | DNS config change | Active |
| MITM proxy | Yes (full content) | None | High | CA cert install | Invasive |
| Browser extension | Yes (perfect) | None | Low | Extension install | Per-browser |

---

## Legal/Ethical Considerations

- **Options 1, 2**: Passive observation of metadata already visible to the OS. Same legal posture as the current SNI/DNS approach. No content inspection.
- **Option 3**: Interposes on DNS resolution but does not inspect content. Moderate — changing system DNS without user knowledge could be viewed unfavorably.
- **Option 4**: Decrypts encrypted traffic. Requires explicit organizational policy and user notice in most jurisdictions. Some regulations (GDPR, ECPA) impose specific requirements on TLS interception.
- **Option 5**: Observes browser activity with user-installed extension. Standard browser permission model applies.
