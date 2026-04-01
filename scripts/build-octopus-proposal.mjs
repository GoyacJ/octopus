import { execFileSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'

const root = '/Users/goya/Work/weilaizhihuigu/super-agent/octopus'
const sourceDocx = '/Users/goya/Desktop/Octopus_产品介绍方案书.docx'
const outputDir = path.join(root, 'output', 'docx-assets')
const outputHtml = path.join(outputDir, 'Octopus_产品介绍方案书_增强版.html')
const outputDocx = '/Users/goya/Desktop/Octopus_产品介绍方案书_增强版.docx'

const assets = {
  valueLoop: path.join(outputDir, 'value-loop.png'),
  platformLayers: path.join(outputDir, 'platform-layers.png'),
  governanceFlow: path.join(outputDir, 'governance-flow.png'),
  dashboard: path.join(root, 'output', 'playwright', 'dashboard.png'),
  conversation: path.join(root, 'output', 'playwright', 'conversation.png'),
  knowledge: path.join(root, 'output', 'playwright', 'knowledge.png'),
  trace: path.join(root, 'output', 'playwright', 'trace.png'),
}

for (const file of [sourceDocx, ...Object.values(assets)]) {
  if (!fs.existsSync(file)) {
    throw new Error(`Missing required file: ${file}`)
  }
}

const originalHtml = execFileSync('textutil', ['-convert', 'html', '-stdout', sourceDocx], {
  encoding: 'utf8',
  maxBuffer: 20 * 1024 * 1024,
})

const extraStyles = `
  body {
    margin: 0;
    padding: 40px 56px;
    background: #f6f8fc;
    color: #213047;
    font-family: "PingFang SC", "Helvetica Neue", Arial, sans-serif;
    line-height: 1.8;
  }
  .proposal {
    max-width: 880px;
    margin: 0 auto;
    padding: 42px 48px 56px;
    background: #ffffff;
    border: 1px solid #e1e8f3;
    border-radius: 22px;
    box-shadow: 0 18px 50px rgba(60, 88, 146, 0.10);
  }
  p.p1, p.p2, p.p3, p.p4, li.li1 {
    font-family: "PingFang SC", "Helvetica Neue", Arial, sans-serif;
    color: #213047;
  }
  p.p1, li.li1 {
    font-size: 16px;
    line-height: 1.86;
    margin: 0 0 10px 0;
  }
  p.p2 {
    min-height: 10px;
    margin: 8px 0;
  }
  p.p3, p.p4 {
    margin: 8px 0 16px 22px;
    padding-left: 16px;
    border-left: 4px solid #d8e5ff;
    font-size: 18px;
    line-height: 1.8;
    color: #38527c;
  }
  ul.ul1 {
    margin: 4px 0 14px 18px;
    padding-left: 22px;
  }
  li.li1 {
    margin-bottom: 6px;
  }
  .hero-title {
    margin-bottom: 18px !important;
    font-size: 28px !important;
    line-height: 1.35 !important;
    color: #12233d !important;
  }
  .section-title {
    margin-top: 28px !important;
    margin-bottom: 14px !important;
    padding-top: 8px;
    font-size: 24px !important;
    color: #12233d !important;
    border-top: 1px solid #edf2fa;
  }
  .section-title:first-of-type {
    border-top: none;
  }
  .subheading {
    margin-top: 18px !important;
    margin-bottom: 8px !important;
    font-size: 19px !important;
    color: #16345f !important;
  }
  .minor-heading {
    margin-top: 14px !important;
    margin-bottom: 6px !important;
    font-size: 17px !important;
    color: #19385f !important;
  }
  .callout {
    margin: 14px 0 22px;
    padding: 18px 22px;
    border-radius: 18px;
    background: linear-gradient(180deg, #f7fbff 0%, #eff6ff 100%);
    border: 1px solid #dbe8fb;
  }
  .callout-title {
    margin: 0 0 8px;
    font-size: 15px;
    font-weight: 700;
    letter-spacing: 0.14em;
    color: #4d8dff;
  }
  .callout p {
    margin: 0;
    font-size: 15px;
    line-height: 1.8;
    color: #445571;
  }
  .mini-grid {
    margin-top: 12px;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }
  .mini-card {
    padding: 12px 14px;
    border-radius: 14px;
    background: rgba(255,255,255,0.85);
    border: 1px solid #dfe8f5;
  }
  .mini-card strong {
    display: block;
    margin-bottom: 4px;
    font-size: 14px;
    color: #18365f;
  }
  .mini-card span {
    font-size: 13px;
    color: #596a84;
    line-height: 1.7;
  }
  .figure-block {
    margin: 20px 0 28px;
    padding: 14px 14px 10px;
    border-radius: 20px;
    background: #f8fbff;
    border: 1px solid #dce7f6;
  }
  .figure-block img {
    display: block;
    width: 100%;
    height: auto;
    border-radius: 14px;
    border: 1px solid #e1eaf6;
    background: #fff;
  }
  .figure-block figcaption {
    margin-top: 10px;
    font-size: 13px;
    line-height: 1.75;
    color: #5b6c85;
  }
  .insert-note {
    margin: 10px 0 18px;
    padding: 12px 16px;
    border-left: 4px solid #4d8dff;
    background: #f7fbff;
    color: #465976;
    font-size: 14px;
    line-height: 1.8;
  }
`

function figure(src, caption) {
  return `
<figure class="figure-block">
  <img src="${src}" />
  <figcaption>${caption}</figcaption>
</figure>`
}

function insertAfter(html, marker, insertion, label) {
  if (!html.includes(marker)) {
    throw new Error(`Marker not found for ${label}`)
  }
  return html.replace(marker, `${marker}\n${insertion}`)
}

let html = originalHtml

html = html.replace('</style>', `${extraStyles}\n  </style>`)
html = html.replace('<body>', '<body><div class="proposal">')
html = html.replace('</body>', '</div></body>')

html = html.replace(
  /<p class="p1"><b>《Octopus 平台产品介绍方案书》<\/b><\/p>/,
  '<p class="p1 hero-title"><b>《Octopus 平台产品介绍方案书》</b></p>',
)
html = html.replace(
  /<p class="p1"><b>([一二三四五六七八九十]+、[^<]+)<\/b><\/p>/g,
  '<p class="p1 section-title"><b>$1</b></p>',
)
html = html.replace(
  /<p class="p1"><b>(\d+\.\d+ [^<]+)<\/b><\/p>/g,
  '<p class="p1 subheading"><b>$1</b></p>',
)
html = html.replace(
  /<p class="p1"><b>(（\d+）[^<]+)<\/b><\/p>/g,
  '<p class="p1 minor-heading"><b>$1</b></p>',
)
html = html.replace(
  /<p class="p1"><b>(\d+）[^<]+)<\/b><\/p>/g,
  '<p class="p1 minor-heading"><b>$1</b></p>',
)

const executiveSummary = `
<div class="callout">
  <div class="callout-title">MANAGEMENT SNAPSHOT</div>
  <p>从管理层视角看，Octopus 的核心价值在于：把 AI 从员工分散使用的辅助工具，升级为企业内可运行、可治理、可持续沉淀的正式生产体系。</p>
  <div class="mini-grid">
    <div class="mini-card"><strong>降本增效</strong><span>把复杂、多角色、长流程任务交给可持续运行的智能体体系推进。</span></div>
    <div class="mini-card"><strong>统一治理</strong><span>将授权、预算、审批、审计统一纳入企业管理边界。</span></div>
    <div class="mini-card"><strong>平台升级</strong><span>让执行数据、成果物与知识资产持续沉淀复用，而不是留在单次对话里。</span></div>
  </div>
</div>`

html = insertAfter(
  html,
  '<p class="p1 hero-title"><b>《Octopus 平台产品介绍方案书》</b></p>',
  executiveSummary,
  'executive summary',
)

html = insertAfter(
  html,
  '<p class="p1 subheading"><b>2.2 核心价值主张</b></p>',
  `<div class="insert-note">对管理层而言，Octopus 的价值不是“增加一个 AI 工具”，而是同时解决效率、治理与平台化三个问题，让 AI 真正进入正式经营体系。</div>
${figure(assets.valueLoop, '图示说明：Octopus 将降本增效、统一治理和平台升级收敛到同一平台闭环中，避免 AI 只停留在个人零散使用阶段。')}`,
  'section 2.2 visual',
)

html = insertAfter(
  html,
  '<p class="p1 section-title"><b>三、核心产品架构</b></p>',
  `${figure(assets.platformLayers, '图示说明：前台是工作表面，后台是运行、知识、治理与执行能力，观测层横切所有环节，使管理层可以看见执行状态、风险和沉淀结果。')}`,
  'section 3 visual',
)

html = insertAfter(
  html,
  '<li class="li1">支持长时间运行</li>',
  `${figure(assets.conversation, '产品示意：Conversation 是业务方与智能体协作的第一入口，任务目标、上下文、进度与下一步动作可以在同一界面中持续推进。')}`,
  'conversation screenshot',
)

html = insertAfter(
  html,
  '<p class="p1">才能成为正式知识。</p>',
  `${figure(assets.knowledge, '产品示意：Knowledge 将私有记忆、共享知识与候选知识分层管理，使团队经验能够从个人对话中沉淀为组织可复用资产。')}`,
  'knowledge screenshot',
)

html = insertAfter(
  html,
  '<p class="p1 subheading"><b>4.3 自治治理系统（Autonomy Governance）</b></p>',
  `${figure(assets.governanceFlow, '图示说明：Octopus 将业务发起、Agent 执行、Run 编排、审批风控和成果沉淀连接为一条正式链路，治理能力并非外挂，而是伴随全过程。')}`,
  'governance flow',
)

html = insertAfter(
  html,
  '<p class="p1">长期运行智能体。</p>',
  `${figure(assets.dashboard, '产品示意：Dashboard 让管理者快速掌握工作区状态、活跃项目、待处理事项与当前重点任务，是连接执行与管理的统一观察入口。')}`,
  'dashboard screenshot',
)

html = insertAfter(
  html,
  '<li class="li1">审批分级机制</li>',
  `${figure(assets.trace, '产品示意：Trace 将运行状态、阻塞原因、责任主体与下一步动作显性化，帮助企业在扩大 AI 使用范围时仍然保持可追踪、可审计。')}
<div class="insert-note">管理层可直接把 AI 行为纳入现有流程管理逻辑中，而不是依赖员工个人判断或不可追溯的外部工具链。</div>`,
  'trace screenshot',
)

html = insertAfter(
  html,
  '<p class="p1 section-title"><b>八、产品优势总结</b></p>',
  '<div class="insert-note">从管理层决策角度，Octopus 的差异化不只是能力多，而是它能在同一平台内同时承接执行效率、风险控制和资产沉淀三类目标，使 AI 从试用型投入转变为可长期经营的能力基础设施。</div>',
  'section 8 note',
)

fs.writeFileSync(outputHtml, html)

execFileSync('textutil', ['-convert', 'docx', outputHtml, '-output', outputDocx], {
  stdio: 'inherit',
})

console.log(`Generated HTML: ${outputHtml}`)
console.log(`Generated DOCX: ${outputDocx}`)
