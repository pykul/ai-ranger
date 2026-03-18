/** Inline SVG icons for AI providers. Renders a small colored icon next to provider names. */

const SIZE = 16;

interface Props {
  provider: string;
  className?: string;
}

export default function ProviderIcon({ provider, className = "" }: Props) {
  const Icon = icons[provider];
  if (!Icon) return <FallbackIcon letter={provider.charAt(0)} className={className} />;
  return <Icon className={className} />;
}

function FallbackIcon({ letter, className }: { letter: string; className?: string }) {
  return (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#94a3b8" />
      <text x="8" y="12" textAnchor="middle" fontSize="10" fontWeight="600" fill="#fff">
        {letter.toUpperCase()}
      </text>
    </svg>
  );
}

type IconFn = (props: { className?: string }) => JSX.Element;

// Each icon is a simplified, recognizable representation of the provider's brand.
const icons: Record<string, IconFn> = {
  openai: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#10a37f" />
      <path d="M8 3.5a4.5 4.5 0 0 1 3.18 7.68l-.01.01A2.5 2.5 0 0 1 8 12.5a2.5 2.5 0 0 1-3.17-1.31A4.5 4.5 0 0 1 8 3.5z" fill="none" stroke="#fff" strokeWidth="1.2" />
    </svg>
  ),

  anthropic: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#d97706" />
      <path d="M5 12L8 4l3 8M6 10h4" stroke="#fff" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round" fill="none" />
    </svg>
  ),

  cursor: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#000" />
      <path d="M5 3l7 5-7 5V3z" fill="#fff" />
    </svg>
  ),

  copilot: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#24292f" />
      <circle cx="6" cy="8" r="1.5" fill="#fff" />
      <circle cx="10" cy="8" r="1.5" fill="#fff" />
      <path d="M4 6.5Q8 3 12 6.5" fill="none" stroke="#fff" strokeWidth="1.2" />
    </svg>
  ),

  github_copilot: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#24292f" />
      <circle cx="6" cy="8" r="1.5" fill="#fff" />
      <circle cx="10" cy="8" r="1.5" fill="#fff" />
      <path d="M4 6.5Q8 3 12 6.5" fill="none" stroke="#fff" strokeWidth="1.2" />
    </svg>
  ),

  gemini: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#4285f4" />
      <path d="M8 3Q12 8 8 13Q4 8 8 3z" fill="#fff" fillOpacity="0.9" />
    </svg>
  ),

  google_gemini: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#4285f4" />
      <path d="M8 3Q12 8 8 13Q4 8 8 3z" fill="#fff" fillOpacity="0.9" />
    </svg>
  ),

  mistral: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#f97316" />
      <rect x="3" y="4" width="3" height="3" rx="0.5" fill="#fff" />
      <rect x="10" y="4" width="3" height="3" rx="0.5" fill="#fff" />
      <rect x="3" y="9" width="3" height="3" rx="0.5" fill="#fff" />
      <rect x="10" y="9" width="3" height="3" rx="0.5" fill="#fff" />
    </svg>
  ),

  cohere: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#39594d" />
      <circle cx="8" cy="8" r="3.5" fill="none" stroke="#d1fae5" strokeWidth="1.5" />
    </svg>
  ),

  huggingface: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#ffd21e" />
      <circle cx="6" cy="7" r="1" fill="#000" />
      <circle cx="10" cy="7" r="1" fill="#000" />
      <path d="M5.5 10Q8 12.5 10.5 10" fill="none" stroke="#000" strokeWidth="1" strokeLinecap="round" />
    </svg>
  ),

  replicate: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#262626" />
      <rect x="4" y="4" width="4" height="4" rx="1" fill="#fff" />
      <rect x="8" y="8" width="4" height="4" rx="1" fill="#fff" opacity="0.6" />
    </svg>
  ),

  together: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#3b82f6" />
      <circle cx="6" cy="8" r="2.5" fill="none" stroke="#fff" strokeWidth="1.2" />
      <circle cx="10" cy="8" r="2.5" fill="none" stroke="#fff" strokeWidth="1.2" />
    </svg>
  ),

  perplexity: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#20b2aa" />
      <path d="M4 12V6l4-3 4 3v6" fill="none" stroke="#fff" strokeWidth="1.2" strokeLinejoin="round" />
      <line x1="8" y1="3" x2="8" y2="12" stroke="#fff" strokeWidth="1.2" />
    </svg>
  ),

  deepseek: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#4f46e5" />
      <path d="M4 8h8M8 4v8" stroke="#fff" strokeWidth="1.5" strokeLinecap="round" />
      <circle cx="8" cy="8" r="3" fill="none" stroke="#fff" strokeWidth="1" />
    </svg>
  ),

  xai: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#000" />
      <path d="M4 4l8 8M12 4l-8 8" stroke="#fff" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  ),

  ai21: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#6d28d9" />
      <text x="8" y="12" textAnchor="middle" fontSize="9" fontWeight="700" fill="#fff">21</text>
    </svg>
  ),

  bedrock: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#ff9900" />
      <path d="M8 3l5 3v4l-5 3-5-3V6z" fill="none" stroke="#fff" strokeWidth="1.2" strokeLinejoin="round" />
    </svg>
  ),

  amazon_bedrock: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#ff9900" />
      <path d="M8 3l5 3v4l-5 3-5-3V6z" fill="none" stroke="#fff" strokeWidth="1.2" strokeLinejoin="round" />
    </svg>
  ),

  azure_openai: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#0078d4" />
      <path d="M4 4h4v4H4zM8 8h4v4H8z" fill="#fff" fillOpacity="0.9" />
    </svg>
  ),

  stability: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#7c3aed" />
      <path d="M4 10Q8 2 12 10" fill="none" stroke="#fff" strokeWidth="1.3" strokeLinecap="round" />
    </svg>
  ),

  ollama: ({ className }) => (
    <svg width={SIZE} height={SIZE} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#1e293b" />
      <circle cx="8" cy="7" r="3.5" fill="none" stroke="#fff" strokeWidth="1.2" />
      <circle cx="8" cy="7" r="1" fill="#fff" />
      <path d="M6 11.5Q8 13 10 11.5" fill="none" stroke="#fff" strokeWidth="1" strokeLinecap="round" />
    </svg>
  ),
};
