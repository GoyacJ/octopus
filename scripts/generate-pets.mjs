import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const petDefs = {
  duck: `
    <circle cx="64" cy="74" r="40" fill="#FFD54F"/>
    <circle cx="64" cy="44" r="30" fill="#FFD54F"/>
    <circle cx="50" cy="38" r="4" fill="#1F2937"/>
    <circle cx="78" cy="38" r="4" fill="#1F2937"/>
    <path d="M54 50 Q 64 60 74 50 Z" fill="#FF8F00"/>
  `,
  goose: `
    <path d="M64 20 Q 74 20 74 40 L 74 60 Q 94 60 94 80 Q 94 104 64 104 Q 34 104 34 80 Q 34 60 54 60 L 54 40 Q 54 20 64 20 Z" fill="#FFFFFF"/>
    <circle cx="58" cy="30" r="3" fill="#1F2937"/>
    <path d="M40 34 Q 50 38 40 42 Z" fill="#F57C00"/>
  `,
  blob: `
    <path d="M64 24 C 94 24 104 54 104 84 C 104 104 24 104 24 84 C 24 54 34 24 64 24 Z" fill="#B39DDB"/>
    <circle cx="48" cy="60" r="5" fill="#1F2937"/>
    <circle cx="80" cy="60" r="5" fill="#1F2937"/>
    <path d="M58 74 Q 64 80 70 74 Z" fill="#1F2937"/>
  `,
  cat: `
    <polygon points="34,24 50,44 34,64" fill="#FFB74D"/>
    <polygon points="94,24 78,44 94,64" fill="#FFB74D"/>
    <circle cx="64" cy="74" r="44" fill="#FFB74D"/>
    <circle cx="48" cy="60" r="4" fill="#1F2937"/>
    <circle cx="80" cy="60" r="4" fill="#1F2937"/>
    <path d="M60 70 Q 64 74 68 70" stroke="#1F2937" stroke-width="3" fill="none"/>
    <path d="M48 76 Q 30 76 24 70 M 80 76 Q 98 76 104 70" stroke="#1F2937" stroke-width="2" fill="none"/>
  `,
  dragon: `
    <path d="M64 24 C 94 24 94 54 84 64 C 104 84 94 104 64 104 C 34 104 24 84 44 64 C 34 54 34 24 64 24 Z" fill="#81C784"/>
    <polygon points="64,14 54,24 74,24" fill="#388E3C"/>
    <polygon points="64,104 54,114 74,114" fill="#388E3C"/>
    <circle cx="50" cy="44" r="4" fill="#1F2937"/>
    <circle cx="78" cy="44" r="4" fill="#1F2937"/>
    <path d="M54 60 Q 64 64 74 60" stroke="#1F2937" stroke-width="3" fill="none"/>
  `,
  octopus: `
    <path d="M34 64 C 34 34 94 34 94 64 L 94 84 C 94 94 34 94 34 84 Z" fill="#F06292"/>
    <path d="M34 84 Q 24 104 44 94 M 54 84 Q 54 104 64 94 M 74 84 Q 84 104 84 94 M 94 84 Q 104 104 84 94" stroke="#F06292" stroke-width="12" stroke-linecap="round" fill="none"/>
    <circle cx="48" cy="54" r="5" fill="#1F2937"/>
    <circle cx="80" cy="54" r="5" fill="#1F2937"/>
    <circle cx="64" cy="70" r="6" fill="#1F2937" fill-opacity="0.2"/>
  `,
  owl: `
    <circle cx="64" cy="64" r="44" fill="#A1887F"/>
    <circle cx="46" cy="50" r="16" fill="#FFFFFF"/>
    <circle cx="82" cy="50" r="16" fill="#FFFFFF"/>
    <circle cx="46" cy="50" r="5" fill="#1F2937"/>
    <circle cx="82" cy="50" r="5" fill="#1F2937"/>
    <polygon points="56,64 72,64 64,76" fill="#FFB300"/>
    <path d="M20 64 Q 10 74 24 94 M 108 64 Q 118 74 104 94" stroke="#8D6E63" stroke-width="10" stroke-linecap="round" fill="none"/>
  `,
  penguin: `
    <rect x="34" y="24" width="60" height="80" rx="30" fill="#1F2937"/>
    <rect x="44" y="34" width="40" height="60" rx="20" fill="#FFFFFF"/>
    <circle cx="50" cy="44" r="4" fill="#1F2937"/>
    <circle cx="78" cy="44" r="4" fill="#1F2937"/>
    <polygon points="58,54 70,54 64,62" fill="#FFCA28"/>
    <path d="M34 54 Q 14 64 24 74 M 94 54 Q 114 64 104 74" stroke="#1F2937" stroke-width="10" stroke-linecap="round" fill="none"/>
  `,
  turtle: `
    <circle cx="64" cy="74" r="34" fill="#4CAF50"/>
    <circle cx="64" cy="34" r="20" fill="#81C784"/>
    <circle cx="20" cy="74" r="14" fill="#81C784"/>
    <circle cx="108" cy="74" r="14" fill="#81C784"/>
    <circle cx="44" cy="104" r="14" fill="#81C784"/>
    <circle cx="84" cy="104" r="14" fill="#81C784"/>
    <circle cx="56" cy="30" r="3" fill="#1F2937"/>
    <circle cx="72" cy="30" r="3" fill="#1F2937"/>
    <path d="M44 64 L 84 64 M 64 44 L 64 84" stroke="#388E3C" stroke-width="4" stroke-linecap="round" fill="none"/>
  `,
  snail: `
    <path d="M24 94 L 104 94 Q 104 84 94 84 L 24 84 Z" fill="#FFCC80"/>
    <circle cx="24" cy="74" r="4" fill="#FFCC80"/>
    <circle cx="24" cy="64" r="3" fill="#1F2937"/>
    <circle cx="64" cy="64" r="30" fill="#8D6E63"/>
    <circle cx="64" cy="64" r="20" fill="#A1887F" stroke="#795548" stroke-width="3"/>
    <path d="M64 44 Q 84 44 84 64" stroke="#795548" stroke-width="3" fill="none"/>
  `,
  ghost: `
    <path d="M34 64 C 34 24 94 24 94 64 L 94 104 Q 84 94 74 104 Q 64 94 54 104 Q 44 94 34 104 Z" fill="#F5F5F5"/>
    <circle cx="50" cy="54" r="6" fill="#1F2937"/>
    <circle cx="78" cy="54" r="6" fill="#1F2937"/>
    <ellipse cx="64" cy="74" rx="8" ry="12" fill="#1F2937"/>
  `,
  axolotl: `
    <circle cx="64" cy="64" r="40" fill="#F8BBD0"/>
    <path d="M24 64 Q 14 54 24 44 M 24 64 Q 14 74 24 84" stroke="#F48FB1" stroke-width="6" stroke-linecap="round" fill="none"/>
    <path d="M104 64 Q 114 54 104 44 M 104 64 Q 114 74 104 84" stroke="#F48FB1" stroke-width="6" stroke-linecap="round" fill="none"/>
    <circle cx="44" cy="60" r="4" fill="#1F2937"/>
    <circle cx="84" cy="60" r="4" fill="#1F2937"/>
    <path d="M54 74 Q 64 84 74 74" stroke="#1F2937" stroke-width="3" fill="none"/>
  `,
  capybara: `
    <rect x="24" y="44" width="80" height="60" rx="30" fill="#8D6E63"/>
    <circle cx="44" cy="34" r="10" fill="#795548"/>
    <circle cx="34" cy="60" r="4" fill="#1F2937"/>
    <path d="M24 70 Q 30 74 28 80" stroke="#1F2937" stroke-width="3" fill="none"/>
    <rect x="64" y="44" width="20" height="10" rx="5" fill="#5D4037"/>
  `,
  cactus: `
    <rect x="44" y="24" width="40" height="80" rx="20" fill="#66BB6A"/>
    <path d="M44 54 Q 24 54 24 44 M 84 64 Q 104 64 104 44" stroke="#4CAF50" stroke-width="12" stroke-linecap="round" fill="none"/>
    <circle cx="54" cy="44" r="3" fill="#1F2937"/>
    <circle cx="74" cy="44" r="3" fill="#1F2937"/>
    <circle cx="64" cy="54" r="4" fill="#1F2937" fill-opacity="0.3"/>
    <path d="M60 20 L 68 20" stroke="#EF5350" stroke-width="6" stroke-linecap="round" fill="none"/>
  `,
  robot: `
    <rect x="34" y="34" width="60" height="50" rx="10" fill="#90A4AE"/>
    <rect x="44" y="84" width="40" height="20" rx="4" fill="#78909C"/>
    <circle cx="64" cy="24" r="6" fill="#EF5350"/>
    <line x1="64" y1="24" x2="64" y2="34" stroke="#90A4AE" stroke-width="4"/>
    <rect x="44" y="44" width="16" height="10" rx="2" fill="#81D4FA"/>
    <rect x="68" y="44" width="16" height="10" rx="2" fill="#81D4FA"/>
    <path d="M50 70 L 78 70" stroke="#455A64" stroke-width="4" stroke-dasharray="8 4" stroke-linecap="round"/>
  `,
  rabbit: `
    <ellipse cx="44" cy="34" rx="10" ry="24" fill="#FAFAFA"/>
    <ellipse cx="84" cy="34" rx="10" ry="24" fill="#FAFAFA"/>
    <ellipse cx="44" cy="34" rx="4" ry="14" fill="#F48FB1"/>
    <ellipse cx="84" cy="34" rx="4" ry="14" fill="#F48FB1"/>
    <circle cx="64" cy="74" r="36" fill="#FAFAFA"/>
    <circle cx="50" cy="64" r="4" fill="#1F2937"/>
    <circle cx="78" cy="64" r="4" fill="#1F2937"/>
    <path d="M60 74 L 64 78 L 68 74" stroke="#F48FB1" stroke-width="3" fill="none"/>
  `,
  mushroom: `
    <path d="M64 24 C 94 24 104 64 104 64 L 24 64 C 24 64 34 24 64 24 Z" fill="#EF5350"/>
    <circle cx="50" cy="40" r="6" fill="#FFFFFF"/>
    <circle cx="78" cy="48" r="8" fill="#FFFFFF"/>
    <circle cx="64" cy="30" r="4" fill="#FFFFFF"/>
    <rect x="44" y="64" width="40" height="40" rx="10" fill="#FFE0B2"/>
    <circle cx="54" cy="74" r="3" fill="#1F2937"/>
    <circle cx="74" cy="74" r="3" fill="#1F2937"/>
    <path d="M60 84 Q 64 88 68 84" stroke="#1F2937" stroke-width="2" fill="none"/>
  `,
  chonk: `
    <rect x="24" y="24" width="80" height="80" rx="40" fill="#BDBDBD"/>
    <circle cx="44" cy="44" r="5" fill="#1F2937"/>
    <circle cx="84" cy="44" r="5" fill="#1F2937"/>
    <path d="M54 54 Q 64 64 74 54" stroke="#1F2937" stroke-width="4" fill="none"/>
    <path d="M34 104 L 34 114 M 94 104 L 94 114" stroke="#BDBDBD" stroke-width="12" stroke-linecap="round"/>
  `
};

async function main() {
  const targetDir = path.resolve(__dirname, '../packages/assets/pets');
  await fs.mkdir(targetDir, { recursive: true });

  for (const [name, content] of Object.entries(petDefs)) {
    const svgContent = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 128" fill="none">
  <g class="pet-body">
${content}
  </g>
</svg>
`;
    const filePath = path.join(targetDir, `${name}.svg`);
    await fs.writeFile(filePath, svgContent, 'utf-8');
    console.log(`Generated ${name}.svg`);
  }
}

main().catch(console.error);
