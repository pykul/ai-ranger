/**
 * Provider brand icons for the dashboard.
 *
 * Uses real brand SVG paths from Simple Icons (https://simpleicons.org/)
 * where available.  Each icon is rendered at 16x16 with the brand's
 * official colour as the fill inside a rounded-rect background.
 *
 * Providers without a Simple Icons entry use a recognisable styled
 * fallback.  Completely unknown providers get a letter avatar.
 */

const S = 16; // rendered size in px

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
    <svg width={S} height={S} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#94a3b8" />
      <text x="8" y="12" textAnchor="middle" fontSize="10" fontWeight="600" fill="#fff">
        {letter.toUpperCase()}
      </text>
    </svg>
  );
}

/** Shorthand: brand path inside a coloured rounded-rect. viewBox is 0 0 24 24 (Simple Icons standard). */
function BrandIcon({ bg, fill, d, className }: { bg: string; fill: string; d: string; className?: string }) {
  // Nest an inner <svg> with a shifted viewBox so the 24x24 icon is centred
  // inside the 16x16 square with 3px padding on each side.
  return (
    <svg width={S} height={S} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill={bg} />
      <svg x="2" y="2" width="12" height="12" viewBox="0 0 24 24">
        <path d={d} fill={fill} />
      </svg>
    </svg>
  );
}

type IconFn = (props: { className?: string }) => JSX.Element;

// --- Real brand paths (from Simple Icons, CC0 / public domain) ---

const OPENAI_D = "M22.2819 9.8211a5.9847 5.9847 0 0 0-.5157-4.9108 6.0462 6.0462 0 0 0-6.5098-2.9A6.0651 6.0651 0 0 0 4.9807 4.1818a5.9847 5.9847 0 0 0-3.9977 2.9 6.0462 6.0462 0 0 0 .7427 7.0966 5.98 5.98 0 0 0 .511 4.9107 6.051 6.051 0 0 0 6.5146 2.9001A5.9847 5.9847 0 0 0 13.2599 24a6.0557 6.0557 0 0 0 5.7718-4.2058 5.9894 5.9894 0 0 0 3.9977-2.9001 6.0557 6.0557 0 0 0-.7475-7.0729zm-9.022 12.6081a4.4755 4.4755 0 0 1-2.8764-1.0408l.1419-.0804 4.7783-2.7582a.7948.7948 0 0 0 .3927-.6813v-6.7369l2.02 1.1686a.071.071 0 0 1 .038.052v5.5826a4.504 4.504 0 0 1-4.4945 4.4944zm-9.6607-4.1254a4.4708 4.4708 0 0 1-.5346-3.0137l.142.0852 4.783 2.7582a.7712.7712 0 0 0 .7806 0l5.8428-3.3685v2.3324a.0804.0804 0 0 1-.0332.0615L9.74 19.9502a4.4992 4.4992 0 0 1-6.1408-1.6464zM2.3408 7.8956a4.485 4.485 0 0 1 2.3655-1.9728V11.6a.7664.7664 0 0 0 .3879.6765l5.8144 3.3543-2.0201 1.1685a.0757.0757 0 0 1-.071 0l-4.8303-2.7865A4.504 4.504 0 0 1 2.3408 7.872zm16.5963 3.8558L13.1038 8.364 15.1192 7.2a.0757.0757 0 0 1 .071 0l4.8303 2.7913a4.4944 4.4944 0 0 1-.6765 8.1042v-5.6772a.79.79 0 0 0-.407-.667zm2.0107-3.0231l-.142-.0852-4.7735-2.7818a.7759.7759 0 0 0-.7854 0L9.409 9.2297V6.8974a.0662.0662 0 0 1 .0284-.0615l4.8303-2.7866a4.4992 4.4992 0 0 1 6.6802 4.66zM8.3065 12.863l-2.02-1.1638a.0804.0804 0 0 1-.038-.0567V6.0742a4.4992 4.4992 0 0 1 7.3757-3.4537l-.142.0805L8.704 5.459a.7948.7948 0 0 0-.3927.6813zm1.0976-2.3654l2.602-1.4998 2.6069 1.4998v2.9994l-2.5974 1.4997-2.6067-1.4997Z";
const ANTHROPIC_D = "M17.3041 3.541h-3.6718l6.696 16.918H24Zm-10.6082 0L0 20.459h3.7442l1.3693-3.5527h7.0052l1.3693 3.5528h3.7442L10.5363 3.5409Zm-.3712 10.2232 2.2914-5.9456 2.2914 5.9456Z";
const CURSOR_D = "M11.503.131 1.891 5.678a.84.84 0 0 0-.42.726v11.188c0 .3.162.575.42.724l9.609 5.55a1 1 0 0 0 .998 0l9.61-5.55a.84.84 0 0 0 .42-.724V6.404a.84.84 0 0 0-.42-.726L12.497.131a1.01 1.01 0 0 0-.996 0M2.657 6.338h18.55c.263 0 .43.287.297.515L12.23 22.918c-.062.107-.229.064-.229-.06V12.335a.59.59 0 0 0-.295-.51l-9.11-5.257c-.109-.063-.064-.23.061-.23";
const COPILOT_D = "M23.922 16.997C23.061 18.492 18.063 22.02 12 22.02 5.937 22.02.939 18.492.078 16.997A.641.641 0 0 1 0 16.741v-2.869a.883.883 0 0 1 .053-.22c.372-.935 1.347-2.292 2.605-2.656.167-.429.414-1.055.644-1.517a10.098 10.098 0 0 1-.052-1.086c0-1.331.282-2.499 1.132-3.368.397-.406.89-.717 1.474-.952C7.255 2.937 9.248 1.98 11.978 1.98c2.731 0 4.767.957 6.166 2.093.584.235 1.077.546 1.474.952.85.869 1.132 2.037 1.132 3.368 0 .368-.014.733-.052 1.086.23.462.477 1.088.644 1.517 1.258.364 2.233 1.721 2.605 2.656a.841.841 0 0 1 .053.22v2.869a.641.641 0 0 1-.078.256Zm-11.75-5.992h-.344a4.359 4.359 0 0 1-.355.508c-.77.947-1.918 1.492-3.508 1.492-1.725 0-2.989-.359-3.782-1.259a2.137 2.137 0 0 1-.085-.104L4 11.746v6.585c1.435.779 4.514 2.179 8 2.179 3.486 0 6.565-1.4 8-2.179v-6.585l-.098-.104s-.033.045-.085.104c-.793.9-2.057 1.259-3.782 1.259-1.59 0-2.738-.545-3.508-1.492a4.359 4.359 0 0 1-.355-.508Zm2.328 3.25c.549 0 1 .451 1 1v2c0 .549-.451 1-1 1-.549 0-1-.451-1-1v-2c0-.549.451-1 1-1Zm-5 0c.549 0 1 .451 1 1v2c0 .549-.451 1-1 1-.549 0-1-.451-1-1v-2c0-.549.451-1 1-1Zm3.313-6.185c.136 1.057.403 1.913.878 2.497.442.544 1.134.938 2.344.938 1.573 0 2.292-.337 2.657-.751.384-.435.558-1.15.558-2.361 0-1.14-.243-1.847-.705-2.319-.477-.488-1.319-.862-2.824-1.025-1.487-.161-2.192.138-2.533.529-.269.307-.437.808-.438 1.578v.021c0 .265.021.562.063.893Zm-1.626 0c.042-.331.063-.628.063-.894v-.02c-.001-.77-.169-1.271-.438-1.578-.341-.391-1.046-.69-2.533-.529-1.505.163-2.347.537-2.824 1.025-.462.472-.705 1.179-.705 2.319 0 1.211.175 1.926.558 2.361.365.414 1.084.751 2.657.751 1.21 0 1.902-.394 2.344-.938.475-.584.742-1.44.878-2.497Z";
const GEMINI_D = "M11.04 19.32Q12 21.51 12 24q0-2.49.93-4.68.96-2.19 2.58-3.81t3.81-2.55Q21.51 12 24 12q-2.49 0-4.68-.93a12.3 12.3 0 0 1-3.81-2.58 12.3 12.3 0 0 1-2.58-3.81Q12 2.49 12 0q0 2.49-.96 4.68-.93 2.19-2.55 3.81a12.3 12.3 0 0 1-3.81 2.58Q2.49 12 0 12q2.49 0 4.68.96 2.19.93 3.81 2.55t2.55 3.81";
const PERPLEXITY_D = "M22.3977 7.0896h-2.3106V.0676l-7.5094 6.3542V.1577h-1.1554v6.1966L4.4904 0v7.0896H1.6023v10.3976h2.8882V24l6.932-6.3591v6.2005h1.1554v-6.0469l6.9318 6.1807v-6.4879h2.8882V7.0896zm-3.4657-4.531v4.531h-5.355l5.355-4.531zm-13.2862.0676 4.8691 4.4634H5.6458V2.6262zM2.7576 16.332V8.245h7.8476l-6.1149 6.1147v1.9723H2.7576zm2.8882 5.0404v-3.8852h.0001v-2.6488l5.7763-5.7764v7.0111l-5.7764 5.2993zm12.7086.0248-5.7766-5.1509V9.0618l5.7766 5.7766v6.5588zm2.8882-5.0652h-1.733v-1.9723L13.3948 8.245h7.8478v8.087z";
const X_D = "M14.234 10.162 22.977 0h-2.072l-7.591 8.824L7.251 0H.258l9.168 13.343L.258 24H2.33l8.016-9.318L16.749 24h6.993zm-2.837 3.299-.929-1.329L3.076 1.56h3.182l5.965 8.532.929 1.329 7.754 11.09h-3.182z";

const icons: Record<string, IconFn> = {
  openai: ({ className }) => (
    <BrandIcon bg="#10a37f" fill="#fff" d={OPENAI_D} className={className} />
  ),

  anthropic: ({ className }) => (
    <BrandIcon bg="#191919" fill="#d4a27f" d={ANTHROPIC_D} className={className} />
  ),

  cursor: ({ className }) => (
    <BrandIcon bg="#000" fill="#fff" d={CURSOR_D} className={className} />
  ),

  copilot: ({ className }) => (
    <BrandIcon bg="#24292f" fill="#fff" d={COPILOT_D} className={className} />
  ),

  github_copilot: ({ className }) => (
    <BrandIcon bg="#24292f" fill="#fff" d={COPILOT_D} className={className} />
  ),

  gemini: ({ className }) => (
    <BrandIcon bg="#fff" fill="#8E75B2" d={GEMINI_D} className={className} />
  ),

  google_gemini: ({ className }) => (
    <BrandIcon bg="#fff" fill="#8E75B2" d={GEMINI_D} className={className} />
  ),

  perplexity: ({ className }) => (
    <BrandIcon bg="#fff" fill="#1FB8CD" d={PERPLEXITY_D} className={className} />
  ),

  xai: ({ className }) => (
    <BrandIcon bg="#000" fill="#fff" d={X_D} className={className} />
  ),

  ollama: ({ className }) => (
    <BrandIcon bg="#1e293b" fill="#fff" d="M16.361 10.26a.894.894 0 0 0-.558.47l-.072.148.001.207c0 .193.004.217.059.353.076.193.152.312.291.448.24.238.51.3.872.205a.86.86 0 0 0 .517-.436.752.752 0 0 0 .08-.498c-.064-.453-.33-.782-.724-.897a1.06 1.06 0 0 0-.466 0zm-9.203.005c-.305.096-.533.32-.65.639a1.187 1.187 0 0 0-.06.52c.057.309.31.59.598.667.362.095.632.033.872-.205.14-.136.215-.255.291-.448.055-.136.059-.16.059-.353l.001-.207-.072-.148a.894.894 0 0 0-.565-.472 1.02 1.02 0 0 0-.474.007Zm4.184 2c-.131.071-.223.25-.195.383.031.143.157.288.353.407.105.063.112.072.117.136.004.038-.01.146-.029.243-.02.094-.036.194-.036.222.002.074.07.195.143.253.064.052.076.054.255.059.164.005.198.001.264-.03.169-.082.212-.234.15-.525-.052-.243-.042-.28.087-.355.137-.08.281-.219.324-.314a.365.365 0 0 0-.175-.48.394.394 0 0 0-.181-.033c-.126 0-.207.03-.355.124l-.085.053-.053-.032c-.219-.13-.259-.145-.391-.143a.396.396 0 0 0-.193.032zm.39-2.195c-.373.036-.475.05-.654.086-.291.06-.68.195-.951.328-.94.46-1.589 1.226-1.787 2.114-.04.176-.045.234-.045.53 0 .294.005.357.043.524.264 1.16 1.332 2.017 2.714 2.173.3.033 1.596.033 1.896 0 1.11-.125 2.064-.727 2.493-1.571.114-.226.169-.372.22-.602.039-.167.044-.23.044-.523 0-.297-.005-.355-.045-.531-.288-1.29-1.539-2.304-3.072-2.497a6.873 6.873 0 0 0-.855-.031zm.645.937a3.283 3.283 0 0 1 1.44.514c.223.148.537.458.671.662.166.251.26.508.303.82.02.143.01.251-.043.482-.08.345-.332.705-.672.957a3.115 3.115 0 0 1-.689.348c-.382.122-.632.144-1.525.138-.582-.006-.686-.01-.853-.042-.57-.107-1.022-.334-1.35-.68-.264-.28-.385-.535-.45-.946-.03-.192.025-.509.137-.776.136-.326.488-.73.836-.963.403-.269.934-.46 1.422-.512.187-.02.586-.02.773-.002zm-5.503-11a1.653 1.653 0 0 0-.683.298C5.617.74 5.173 1.666 4.985 2.819c-.07.436-.119 1.04-.119 1.503 0 .544.064 1.24.155 1.721.02.107.031.202.023.208a8.12 8.12 0 0 1-.187.152 5.324 5.324 0 0 0-.949 1.02 5.49 5.49 0 0 0-.94 2.339 6.625 6.625 0 0 0-.023 1.357c.091.78.325 1.438.727 2.04l.13.195-.037.064c-.269.452-.498 1.105-.605 1.732-.084.496-.095.629-.095 1.294 0 .67.009.803.088 1.266.095.555.288 1.143.503 1.534.071.128.243.393.264.407.007.003-.014.067-.046.141a7.405 7.405 0 0 0-.548 1.873c-.062.417-.071.552-.071.991 0 .56.031.832.148 1.279L3.42 24h1.478l-.05-.091c-.297-.552-.325-1.575-.068-2.597.117-.472.25-.819.498-1.296l.148-.29v-.177c0-.165-.003-.184-.057-.293a.915.915 0 0 0-.194-.25 1.74 1.74 0 0 1-.385-.543c-.424-.92-.506-2.286-.208-3.451.124-.486.329-.918.544-1.154a.787.787 0 0 0 .223-.531c0-.195-.07-.355-.224-.522a3.136 3.136 0 0 1-.817-1.729c-.14-.96.114-2.005.69-2.834.563-.814 1.353-1.336 2.237-1.475.199-.033.57-.028.776.01.226.04.367.028.512-.041.179-.085.268-.19.374-.431.093-.215.165-.333.36-.576.234-.29.46-.489.822-.729.413-.27.884-.467 1.352-.561.17-.035.25-.04.569-.04.319 0 .398.005.569.04a4.07 4.07 0 0 1 1.914.997c.117.109.398.457.488.602.034.057.095.177.132.267.105.241.195.346.374.43.14.068.286.082.503.045.343-.058.607-.053.943.016 1.144.23 2.14 1.173 2.581 2.437.385 1.108.276 2.267-.296 3.153-.097.15-.193.27-.333.419-.301.322-.301.722-.001 1.053.493.539.801 1.866.708 3.036-.062.772-.26 1.463-.533 1.854a2.096 2.096 0 0 1-.224.258.916.916 0 0 0-.194.25c-.054.109-.057.128-.057.293v.178l.148.29c.248.476.38.823.498 1.295.253 1.008.231 2.01-.059 2.581a.845.845 0 0 0-.044.098c0 .006.329.009.732.009h.73l.02-.074.036-.134c.019-.076.057-.3.088-.516.029-.217.029-1.016 0-1.258-.11-.875-.295-1.57-.597-2.226-.032-.074-.053-.138-.046-.141.008-.005.057-.074.108-.152.376-.569.607-1.284.724-2.228.031-.26.031-1.378 0-1.628-.083-.645-.182-1.082-.348-1.525a6.083 6.083 0 0 0-.329-.7l-.038-.064.131-.194c.402-.604.636-1.262.727-2.04a6.625 6.625 0 0 0-.024-1.358 5.512 5.512 0 0 0-.939-2.339 5.325 5.325 0 0 0-.95-1.02 8.097 8.097 0 0 1-.186-.152.692.692 0 0 1 .023-.208c.208-1.087.201-2.443-.017-3.503-.19-.924-.535-1.658-.98-2.082-.354-.338-.716-.482-1.15-.455-.996.059-1.8 1.205-2.116 3.01a6.805 6.805 0 0 0-.097.726c0 .036-.007.066-.015.066a.96.96 0 0 1-.149-.078A4.857 4.857 0 0 0 12 3.03c-.832 0-1.687.243-2.456.698a.958.958 0 0 1-.148.078c-.008 0-.015-.03-.015-.066a6.71 6.71 0 0 0-.097-.725C8.997 1.392 8.337.319 7.46.048a2.096 2.096 0 0 0-.585-.041Zm.293 1.402c.248.197.523.759.682 1.388.03.113.06.244.069.292.007.047.026.152.041.233.067.365.098.76.102 1.24l.002.475-.12.175-.118.178h-.278c-.324 0-.646.041-.954.124l-.238.06c-.033.007-.038-.003-.057-.144a8.438 8.438 0 0 1 .016-2.323c.124-.788.413-1.501.696-1.711.067-.05.079-.049.157.013zm9.825-.012c.17.126.358.46.498.888.28.854.36 2.028.212 3.145-.019.14-.024.151-.057.144l-.238-.06a3.693 3.693 0 0 0-.954-.124h-.278l-.119-.178-.119-.175.002-.474c.004-.669.066-1.19.214-1.772.157-.623.434-1.185.68-1.382.078-.062.09-.063.159-.012z" className={className} />
  ),

  microsoft_copilot: ({ className }) => (
    <svg width={S} height={S} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#fff" />
      <image href="data:image/png;base64,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"
        x="1" y="1" width="14" height="14" clipPath="inset(0 round 2px)" />
    </svg>
  ),

  // -- Styled fallbacks for providers not on Simple Icons --

  mistral: ({ className }) => (
    <svg width={S} height={S} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#f97316" />
      <rect x="2" y="3" width="3.5" height="3.5" fill="#000" /><rect x="6.25" y="3" width="3.5" height="3.5" fill="#f97316" /><rect x="10.5" y="3" width="3.5" height="3.5" fill="#000" />
      <rect x="2" y="6.25" width="3.5" height="3.5" fill="#000" /><rect x="6.25" y="6.25" width="3.5" height="3.5" fill="#000" /><rect x="10.5" y="6.25" width="3.5" height="3.5" fill="#000" />
      <rect x="2" y="9.5" width="3.5" height="3.5" fill="#000" /><rect x="6.25" y="9.5" width="3.5" height="3.5" fill="#f97316" /><rect x="10.5" y="9.5" width="3.5" height="3.5" fill="#000" />
    </svg>
  ),

  cohere: ({ className }) => (
    <svg width={S} height={S} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#39594d" />
      <path d="M5 8a3 3 0 1 1 6 0" fill="none" stroke="#d1fae5" strokeWidth="2.5" strokeLinecap="round" />
    </svg>
  ),

  huggingface: ({ className }) => (
    <BrandIcon bg="#FFD21E" fill="#000" d="M12.025 1.13c-5.77 0-10.449 4.647-10.449 10.378 0 1.112.178 2.181.503 3.185.064-.222.203-.444.416-.577a.96.96 0 0 1 .524-.15c.293 0 .584.124.84.284.278.173.48.408.71.694.226.282.458.611.684.951v-.014c.017-.324.106-.622.264-.874s.403-.487.762-.543c.3-.047.596.06.787.203s.31.313.4.467c.15.257.212.468.233.542.01.026.653 1.552 1.657 2.54.616.605 1.01 1.223 1.082 1.912.055.537-.096 1.059-.38 1.572.637.121 1.294.187 1.967.187.657 0 1.298-.063 1.921-.178-.287-.517-.44-1.041-.384-1.581.07-.69.465-1.307 1.081-1.913 1.004-.987 1.647-2.513 1.657-2.539.021-.074.083-.285.233-.542.09-.154.208-.323.4-.467a1.08 1.08 0 0 1 .787-.203c.359.056.604.29.762.543s.247.55.265.874v.015c.225-.34.457-.67.683-.952.23-.286.432-.52.71-.694.257-.16.547-.284.84-.285a.97.97 0 0 1 .524.151c.228.143.373.388.43.625l.006.04a10.3 10.3 0 0 0 .534-3.273c0-5.731-4.678-10.378-10.449-10.378M8.327 6.583a1.5 1.5 0 0 1 .713.174 1.487 1.487 0 0 1 .617 2.013c-.183.343-.762-.214-1.102-.094-.38.134-.532.914-.917.71a1.487 1.487 0 0 1 .69-2.803m7.486 0a1.487 1.487 0 0 1 .689 2.803c-.385.204-.536-.576-.916-.71-.34-.12-.92.437-1.103.094a1.487 1.487 0 0 1 .617-2.013 1.5 1.5 0 0 1 .713-.174m-10.68 1.55a.96.96 0 1 1 0 1.921.96.96 0 0 1 0-1.92m13.838 0a.96.96 0 1 1 0 1.92.96.96 0 0 1 0-1.92M8.489 11.458c.588.01 1.965 1.157 3.572 1.164 1.607-.007 2.984-1.155 3.572-1.164.196-.003.305.12.305.454 0 .886-.424 2.328-1.563 3.202-.22-.756-1.396-1.366-1.63-1.32q-.011.001-.02.006l-.044.026-.01.008-.03.024q-.018.017-.035.036l-.032.04a1 1 0 0 0-.058.09l-.014.025q-.049.088-.11.19a1 1 0 0 1-.083.116 1.2 1.2 0 0 1-.173.18q-.035.029-.075.058a1.3 1.3 0 0 1-.251-.243 1 1 0 0 1-.076-.107c-.124-.193-.177-.363-.337-.444-.034-.016-.104-.008-.2.022q-.094.03-.216.087-.06.028-.125.063l-.13.074q-.067.04-.136.086a3 3 0 0 0-.135.096 3 3 0 0 0-.26.219 2 2 0 0 0-.12.121 2 2 0 0 0-.106.128l-.002.002a2 2 0 0 0-.09.132l-.001.001a1.2 1.2 0 0 0-.105.212q-.013.036-.024.073c-1.139-.875-1.563-2.317-1.563-3.203 0-.334.109-.457.305-.454" className={className} />
  ),

  replicate: ({ className }) => (
    <svg width={S} height={S} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#262626" />
      <rect x="3.5" y="3" width="2.5" height="10" rx="0.5" fill="#fff" />
      <rect x="7" y="3" width="2.5" height="10" rx="0.5" fill="#fff" opacity="0.6" />
      <rect x="10.5" y="3" width="2.5" height="10" rx="0.5" fill="#fff" opacity="0.3" />
    </svg>
  ),

  together: ({ className }) => (
    <svg width={S} height={S} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#3b82f6" />
      <circle cx="5.5" cy="6" r="2" fill="#fff" /><circle cx="10.5" cy="6" r="2" fill="#fff" />
      <circle cx="5.5" cy="11" r="2" fill="#fff" opacity="0.6" /><circle cx="10.5" cy="11" r="2" fill="#fff" opacity="0.6" />
    </svg>
  ),

  deepseek: ({ className }) => (
    <svg width={S} height={S} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#4f46e5" />
      <text x="8" y="12.5" textAnchor="middle" fontSize="10" fontWeight="700" fontFamily="system-ui" fill="#fff">DS</text>
    </svg>
  ),

  ai21: ({ className }) => (
    <svg width={S} height={S} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#6d28d9" />
      <text x="8" y="12" textAnchor="middle" fontSize="9" fontWeight="700" fontFamily="system-ui" fill="#fff">21</text>
    </svg>
  ),

  bedrock: ({ className }) => (
    <svg width={S} height={S} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#ff9900" />
      <path d="M8 3l5 3v4l-5 3-5-3V6z" fill="none" stroke="#fff" strokeWidth="1.2" strokeLinejoin="round" />
      <path d="M3 6l5 3 5-3M8 9v4" fill="none" stroke="#fff" strokeWidth="1" />
    </svg>
  ),

  amazon_bedrock: ({ className }) => (
    <svg width={S} height={S} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#ff9900" />
      <path d="M8 3l5 3v4l-5 3-5-3V6z" fill="none" stroke="#fff" strokeWidth="1.2" strokeLinejoin="round" />
      <path d="M3 6l5 3 5-3M8 9v4" fill="none" stroke="#fff" strokeWidth="1" />
    </svg>
  ),

  azure_openai: ({ className }) => (
    <svg width={S} height={S} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#0078d4" />
      <path d="M3 9.5l4.5-7h2L5 9.5H3zm4 0L9.5 5h2L9 9.5H7zm2.5 3L7 9h2.5l2.5 3.5h-2.5z" fill="#fff" />
    </svg>
  ),

  stability: ({ className }) => (
    <svg width={S} height={S} viewBox="0 0 16 16" className={className}>
      <rect width="16" height="16" rx="3" fill="#7c3aed" />
      <text x="8" y="12" textAnchor="middle" fontSize="9" fontWeight="700" fontFamily="system-ui" fill="#fff">SA</text>
    </svg>
  ),
};
