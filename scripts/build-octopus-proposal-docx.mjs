import fs from 'node:fs'
import path from 'node:path'
import { createRequire } from 'node:module'

const builderDir = process.env.DOCX_HELPER_DIR
if (!builderDir) {
  throw new Error('DOCX_HELPER_DIR is required')
}

const require = createRequire(path.join(builderDir, 'package.json'))
const {
  AlignmentType,
  BorderStyle,
  Document,
  HeadingLevel,
  ImageRun,
  LevelFormat,
  Packer,
  Paragraph,
  ShadingType,
  Table,
  TableCell,
  TableRow,
  TextRun,
  WidthType,
} = require('docx')

const root = '/Users/goya/Work/weilaizhihuigu/super-agent/octopus'
const sourceTxt = path.join(root, 'output', 'docx-assets', 'source.txt')
const outputDocx = '/Users/goya/Desktop/Octopus_产品介绍方案书_增强版.docx'

const assets = {
  valueLoop: path.join(root, 'output', 'docx-assets', 'value-loop.png'),
  platformLayers: path.join(root, 'output', 'docx-assets', 'platform-layers.png'),
  governanceFlow: path.join(root, 'output', 'docx-assets', 'governance-flow.png'),
  dashboard: path.join(root, 'output', 'playwright', 'dashboard.png'),
  conversation: path.join(root, 'output', 'playwright', 'conversation.png'),
  knowledge: path.join(root, 'output', 'playwright', 'knowledge.png'),
  trace: path.join(root, 'output', 'playwright', 'trace.png'),
}

const text = fs.readFileSync(sourceTxt, 'utf8').replace(/\r/g, '')
const lines = text.split('\n')

const bodyChildren = []

const border = { style: BorderStyle.SINGLE, size: 1, color: 'D9E3F1' }
const cellBorders = { top: border, bottom: border, left: border, right: border }

function para(text, options = {}) {
  const size = options.size ?? 22
  const color = options.color ?? '213047'
  const children = options.children ?? [new TextRun({ text, size, color, bold: !!options.bold, font: 'Microsoft YaHei' })]
  return new Paragraph({
    children,
    alignment: options.alignment,
    heading: options.heading,
    spacing: options.spacing ?? { after: 160, line: 360 },
    indent: options.indent,
    shading: options.shading,
    border: options.border,
    pageBreakBefore: options.pageBreakBefore,
    keepNext: options.keepNext,
  })
}

function bullet(text) {
  return new Paragraph({
    numbering: { reference: 'bullets', level: 0 },
    spacing: { after: 80, line: 340 },
    children: [new TextRun({ text, size: 22, color: '213047', font: 'Microsoft YaHei' })],
  })
}

function minorHeading(text) {
  return para(text, {
    bold: true,
    size: 24,
    color: '17365D',
    spacing: { before: 140, after: 100, line: 360 },
    keepNext: true,
  })
}

function quoteLine(text) {
  return new Paragraph({
    alignment: AlignmentType.CENTER,
    spacing: { before: 120, after: 200, line: 380 },
    shading: { fill: 'F5F9FF', type: ShadingType.CLEAR },
    border: {
      left: { style: BorderStyle.SINGLE, size: 8, color: 'D9E7FF' },
    },
    children: [new TextRun({ text, size: 28, color: '38527C', bold: true, font: 'Microsoft YaHei' })],
  })
}

function figure(imagePath, caption, width = 720, height = Math.round(width * 0.5625)) {
  const data = fs.readFileSync(imagePath)
  return [
    new Paragraph({
      alignment: AlignmentType.CENTER,
      spacing: { before: 180, after: 80 },
      children: [
        new ImageRun({
          type: 'png',
          data,
          transformation: { width, height },
          altText: {
            title: caption,
            description: caption,
            name: path.basename(imagePath),
          },
        }),
      ],
    }),
    new Paragraph({
      alignment: AlignmentType.CENTER,
      spacing: { after: 180, line: 320 },
      children: [new TextRun({ text: caption, size: 18, color: '5B6C85', font: 'Microsoft YaHei' })],
    }),
  ]
}

function noteBox(title, content) {
  return new Table({
    width: { size: 9638, type: WidthType.DXA },
    columnWidths: [9638],
    rows: [
      new TableRow({
        children: [
          new TableCell({
            width: { size: 9638, type: WidthType.DXA },
            borders: cellBorders,
            shading: { fill: 'F6FAFF', type: ShadingType.CLEAR },
            margins: { top: 120, bottom: 120, left: 160, right: 160 },
            children: [
              para(title, {
                bold: true,
                size: 18,
                color: '4D8DFF',
                spacing: { after: 80, line: 280 },
              }),
              para(content, {
                size: 20,
                color: '445571',
                spacing: { after: 0, line: 360 },
              }),
            ],
          }),
        ],
      }),
    ],
  })
}

function summaryTable() {
  return new Table({
    width: { size: 9638, type: WidthType.DXA },
    columnWidths: [3212, 3213, 3213],
    rows: [
      new TableRow({
        children: [
          summaryCell('降本增效', '把复杂、多角色、长流程任务交给可持续运行的智能体体系推进。'),
          summaryCell('统一治理', '将授权、预算、审批、审计统一纳入企业管理边界。'),
          summaryCell('平台升级', '让执行数据、成果物与知识资产持续沉淀复用，而不是留在单次对话里。'),
        ],
      }),
    ],
  })
}

function summaryCell(title, desc) {
  return new TableCell({
    width: { size: 3212, type: WidthType.DXA },
    borders: cellBorders,
    shading: { fill: 'FBFDFF', type: ShadingType.CLEAR },
    margins: { top: 120, bottom: 120, left: 140, right: 140 },
    children: [
      para(title, {
        bold: true,
        size: 22,
        color: '18365F',
        spacing: { after: 60, line: 300 },
      }),
      para(desc, {
        size: 18,
        color: '596A84',
        spacing: { after: 0, line: 320 },
      }),
    ],
  })
}

bodyChildren.push(
  new Paragraph({
    alignment: AlignmentType.CENTER,
    spacing: { after: 240, line: 420 },
    children: [new TextRun({ text: '《Octopus 平台产品介绍方案书》', size: 34, bold: true, color: '12233D', font: 'Microsoft YaHei' })],
  }),
)

bodyChildren.push(
  noteBox(
    'MANAGEMENT SNAPSHOT',
    '从管理层视角看，Octopus 的核心价值在于：把 AI 从员工分散使用的辅助工具，升级为企业内可运行、可治理、可持续沉淀的正式生产体系。',
  ),
)
bodyChildren.push(new Paragraph({ spacing: { after: 80 } }))
bodyChildren.push(summaryTable())
bodyChildren.push(new Paragraph({ spacing: { after: 220 } }))

for (let i = 1; i < lines.length; i += 1) {
  const raw = lines[i]
  const trimmed = raw.trim()
  if (!trimmed) {
    continue
  }

  if (/^[一二三四五六七八九十]+、/.test(trimmed)) {
    bodyChildren.push(
      para(trimmed, {
        heading: HeadingLevel.HEADING_1,
        bold: true,
        size: 28,
        color: '12233D',
        spacing: { before: 240, after: 160, line: 380 },
      }),
    )
    if (trimmed === '三、核心产品架构') {
      bodyChildren.push(...figure(
        assets.platformLayers,
        '图示说明：前台是工作表面，后台是运行、知识、治理与执行能力，观测层横切所有环节，使管理层可以看见执行状态、风险和沉淀结果。',
      ))
    }
    continue
  }

  if (/^\d+\.\d+ /.test(trimmed)) {
    bodyChildren.push(
      para(trimmed, {
        heading: HeadingLevel.HEADING_2,
        bold: true,
        size: 24,
        color: '16345F',
        spacing: { before: 180, after: 120, line: 360 },
        keepNext: true,
      }),
    )
    if (trimmed === '2.2 核心价值主张') {
      bodyChildren.push(
        noteBox(
          'EXECUTIVE VIEW',
          '对管理层而言，Octopus 的价值不是“增加一个 AI 工具”，而是同时解决效率、治理与平台化三个问题，让 AI 真正进入正式经营体系。',
        ),
      )
      bodyChildren.push(new Paragraph({ spacing: { after: 80 } }))
      bodyChildren.push(...figure(
        assets.valueLoop,
        '图示说明：Octopus 将降本增效、统一治理和平台升级收敛到同一平台闭环中，避免 AI 只停留在个人零散使用阶段。',
      ))
    }
    if (trimmed === '4.3 自治治理系统（Autonomy Governance）') {
      bodyChildren.push(...figure(
        assets.governanceFlow,
        '图示说明：Octopus 将业务发起、Agent 执行、Run 编排、审批风控和成果沉淀连接为一条正式链路，治理能力并非外挂，而是伴随全过程。',
      ))
    }
    continue
  }

  if (/^（\d+）/.test(trimmed) || /^\d+）/.test(trimmed)) {
    bodyChildren.push(minorHeading(trimmed))
    continue
  }

  if (trimmed === '统一的智能体运行与治理平台，而非单点 AI 工具' || trimmed === '可运行、可治理、可协作、可扩展的智能体操作系统') {
    bodyChildren.push(quoteLine(trimmed))
    continue
  }

  const bulletText = raw.replace(/^[\t ]*•[\t ]*/, '').trim()
  if (raw.includes('•') && bulletText) {
    bodyChildren.push(bullet(bulletText))
    if (bulletText === '支持长时间运行') {
      bodyChildren.push(...figure(
        assets.conversation,
        '产品示意：Conversation 是业务方与智能体协作的第一入口，任务目标、上下文、进度与下一步动作可以在同一界面中持续推进。',
        700,
        585,
      ))
    }
    if (bulletText === '审批分级机制') {
      bodyChildren.push(...figure(
        assets.trace,
        '产品示意：Trace 将运行状态、阻塞原因、责任主体与下一步动作显性化，帮助企业在扩大 AI 使用范围时仍然保持可追踪、可审计。',
        700,
        585,
      ))
      bodyChildren.push(
        noteBox(
          'MANAGEMENT NOTE',
          '管理层可直接把 AI 行为纳入现有流程管理逻辑中，而不是依赖员工个人判断或不可追溯的外部工具链。',
        ),
      )
    }
    continue
  }

  if (trimmed === '才能成为正式知识。') {
    bodyChildren.push(para(trimmed))
    bodyChildren.push(...figure(
      assets.knowledge,
      '产品示意：Knowledge 将私有记忆、共享知识与候选知识分层管理，使团队经验能够从个人对话中沉淀为组织可复用资产。',
      700,
      525,
    ))
    continue
  }

  if (trimmed === '长期运行智能体。') {
    bodyChildren.push(para(trimmed))
    bodyChildren.push(...figure(
      assets.dashboard,
      '产品示意：Dashboard 让管理者快速掌握工作区状态、活跃项目、待处理事项与当前重点任务，是连接执行与管理的统一观察入口。',
      700,
      467,
    ))
    continue
  }

  if (trimmed === 'Octopus 相比传统 AI 产品，具备以下核心优势：') {
    bodyChildren.push(
      noteBox(
        'DECISION PERSPECTIVE',
        '从管理层决策角度，Octopus 的差异化不只是能力多，而是它能在同一平台内同时承接执行效率、风险控制和资产沉淀三类目标，使 AI 从试用型投入转变为可长期经营的能力基础设施。',
      ),
    )
    bodyChildren.push(new Paragraph({ spacing: { after: 80 } }))
  }

  bodyChildren.push(para(trimmed))
}

const doc = new Document({
  styles: {
    default: {
      document: {
        run: {
          font: 'Microsoft YaHei',
          size: 22,
          color: '213047',
        },
      },
    },
    paragraphStyles: [
      {
        id: 'Heading1',
        name: 'Heading 1',
        basedOn: 'Normal',
        next: 'Normal',
        quickFormat: true,
        run: { size: 28, bold: true, font: 'Microsoft YaHei', color: '12233D' },
        paragraph: { spacing: { before: 240, after: 160 }, outlineLevel: 0 },
      },
      {
        id: 'Heading2',
        name: 'Heading 2',
        basedOn: 'Normal',
        next: 'Normal',
        quickFormat: true,
        run: { size: 24, bold: true, font: 'Microsoft YaHei', color: '16345F' },
        paragraph: { spacing: { before: 180, after: 120 }, outlineLevel: 1 },
      },
    ],
  },
  numbering: {
    config: [
      {
        reference: 'bullets',
        levels: [
          {
            level: 0,
            format: LevelFormat.BULLET,
            text: '•',
            alignment: AlignmentType.LEFT,
            style: {
              paragraph: {
                indent: { left: 720, hanging: 360 },
              },
            },
          },
        ],
      },
    ],
  },
  sections: [
    {
      properties: {
        page: {
          size: { width: 11906, height: 16838 },
          margin: { top: 1134, right: 1134, bottom: 1134, left: 1134 },
        },
      },
      children: bodyChildren,
    },
  ],
})

const buffer = await Packer.toBuffer(doc)
fs.writeFileSync(outputDocx, buffer)
console.log(`Generated DOCX with embedded images: ${outputDocx}`)
