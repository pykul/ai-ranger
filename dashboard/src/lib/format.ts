/** Format a provider name for display: "openai" -> "OpenAI", "anthropic" -> "Anthropic" */
const providerNames: Record<string, string> = {
  openai: "OpenAI",
  anthropic: "Anthropic",
  cursor: "Cursor",
  copilot: "GitHub Copilot",
  gemini: "Google Gemini",
  mistral: "Mistral",
  cohere: "Cohere",
  huggingface: "Hugging Face",
  replicate: "Replicate",
  together: "Together AI",
  perplexity: "Perplexity",
  deepseek: "DeepSeek",
  xai: "xAI / Grok",
  ai21: "AI21 Labs",
  bedrock: "Amazon Bedrock",
  azure_openai: "Azure OpenAI",
  stability: "Stability AI",
  ollama: "Ollama",
};

export function formatProvider(raw: string): string {
  return providerNames[raw] ?? raw.charAt(0).toUpperCase() + raw.slice(1);
}

/** Format a process name for display: "chrome.exe" -> "Chrome", "cursor" -> "Cursor" */
export function formatProcess(name: string): string {
  const cleaned = name.replace(/\.exe$/i, "");
  return cleaned.charAt(0).toUpperCase() + cleaned.slice(1);
}

/** Format a detection method: "sni" -> "SNI", "dns" -> "DNS", "ip_range" -> "IP Range" */
export function formatDetection(method: string): string {
  switch (method) {
    case "sni": return "SNI";
    case "dns": return "DNS";
    case "ip_range": return "IP Range";
    case "tcp_heuristic": return "TCP";
    default: return method.toUpperCase();
  }
}

/** Format OS type: "windows" -> "Windows", "macos" -> "macOS", "linux" -> "Linux" */
export function formatOS(os: string): string {
  switch (os) {
    case "windows": return "Windows";
    case "macos": return "macOS";
    case "linux": return "Linux";
    default: return os;
  }
}

/** Format a timestamp as relative time: "2 minutes ago", "3 hours ago" */
export function timeAgo(date: string | Date): string {
  const now = Date.now();
  const then = new Date(date).getTime();
  const seconds = Math.floor((now - then) / 1000);

  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} minute${minutes === 1 ? "" : "s"} ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} hour${hours === 1 ? "" : "s"} ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} day${days === 1 ? "" : "s"} ago`;
  const months = Math.floor(days / 30);
  return `${months} month${months === 1 ? "" : "s"} ago`;
}

/** Format a number with commas: 1234567 -> "1,234,567" */
export function formatNumber(n: number): string {
  return n.toLocaleString();
}
