import fs from "node:fs/promises"
import path from "node:path"

import { Presentation, PresentationFile } from "@oai/artifact-tool"

const ROOT = "/Users/goya/Work/weilaizhihuigu/super-agent/octopus"
const OUT_DIR = path.join(ROOT, "output", "slides", "2026-04-23-octopus-product-intro")
const PREVIEW_DIR = path.join(OUT_DIR, "preview")
const OUTPUT_PPTX = path.join(OUT_DIR, "output.pptx")
const SIZE = { width: 1280, height: 720 }

const C = {
  base: "#FBF7F2",
  surface: "#FFFDF9",
  warm: "#FFF2E6",
  paleOrange: "#FFE7D2",
  primary: "#FF7A1A",
  secondary: "#FFC36B",
  ink: "#171717",
  muted: "#6E6257",
  border: "#E7D8C8",
  blueSoft: "#DCE9F8",
  blueCard: "#EEF4FB",
  greenSoft: "#EAF3E8",
  dark: "#171310",
  darkAlt: "#2A1C14",
  darkText: "#F8F4EF",
  darkMuted: "#D8CFC5",
}

const ASSETS = {
  logo: path.join(ROOT, "apps", "website", "public", "logo.png"),
  home: path.join(ROOT, "output", "playwright", "home-page.png"),
  conversation: path.join(ROOT, "apps", "website", "public", "screenshots", "conversation.png"),
  dashboard: path.join(ROOT, "apps", "website", "public", "screenshots", "dashboard.png"),
  agent: path.join(ROOT, "apps", "website", "public", "screenshots", "agent.png"),
  builtin: path.join(ROOT, "apps", "website", "public", "screenshots", "builtin.png"),
  rbac: path.join(ROOT, "apps", "website", "public", "screenshots", "rbac.png"),
}

const imageCache = new Map()

async function main() {
  await fs.mkdir(PREVIEW_DIR, { recursive: true })
  await assertAssets()

  const presentation = Presentation.create({ slideSize: SIZE })

  await buildSlide1(presentation.slides.add())
  await buildSlide2(presentation.slides.add())
  await buildSlide3(presentation.slides.add())
  await buildSlide4(presentation.slides.add())
  await buildSlide5(presentation.slides.add())
  await buildSlide6(presentation.slides.add())
  await buildSlide7(presentation.slides.add())

  const pptx = await PresentationFile.exportPptx(presentation)
  await pptx.save(OUTPUT_PPTX)

  for (const [index, slide] of presentation.slides.items.entries()) {
    const png = await presentation.export({ slide, format: "png", scale: 1 })
    await writeWebBlob(
      path.join(PREVIEW_DIR, `slide-${String(index + 1).padStart(2, "0")}.png`),
      png,
    )
  }

  console.log(`Deck written to ${OUTPUT_PPTX}`)
}

async function assertAssets() {
  for (const filePath of Object.values(ASSETS)) {
    await fs.access(filePath)
  }
}

async function getImageDataUrl(filePath) {
  if (!imageCache.has(filePath)) {
    const data = await fs.readFile(filePath)
    const encoded = `data:image/png;base64,${data.toString("base64")}`
    imageCache.set(filePath, encoded)
  }
  return imageCache.get(filePath)
}

async function writeWebBlob(filePath, blob) {
  const buffer = Buffer.from(await blob.arrayBuffer())
  await fs.writeFile(filePath, buffer)
}

function setBackground(slide, color) {
  slide.background.fill.color = color
}

function addSoftBlob(slide, frame, fill) {
  const blob = slide.shapes.add({ geometry: "ellipse" })
  blob.frame = frame
  blob.fill.color = fill
  blob.line.visible = false
  blob.sendToBack()
  return blob
}

function addCard(slide, frame, options = {}) {
  const shape = slide.shapes.add({ geometry: options.geometry ?? "roundRect" })
  shape.frame = frame
  shape.fill.color = options.fill ?? C.surface
  shape.line.visible = options.line ?? false
  if (options.line) {
    shape.line.fill.color = options.lineColor ?? C.border
    shape.line.width = options.lineWidth ?? 1
  }
  return shape
}

function addText(slide, text, frame, options = {}) {
  const shape = slide.shapes.add({ geometry: options.geometry ?? "rect" })
  shape.frame = frame
  shape.fill.color = options.fill ?? C.base
  shape.line.visible = options.line ?? false
  if (options.line) {
    shape.line.fill.color = options.lineColor ?? C.border
    shape.line.width = options.lineWidth ?? 1
  }
  shape.text = text
  shape.text.fontSize = options.fontSize ?? 20
  shape.text.color = options.color ?? C.ink
  shape.text.bold = options.bold ?? false
  shape.text.italic = options.italic ?? false
  shape.text.alignment = options.align ?? "left"
  shape.text.verticalAlignment = options.valign ?? "top"
  return shape
}

function addChip(slide, text, frame, options = {}) {
  return addText(slide, text, frame, {
    geometry: "roundRect",
    fill: options.fill ?? C.surface,
    color: options.color ?? C.primary,
    fontSize: options.fontSize ?? 14,
    bold: options.bold ?? true,
    align: "center",
    valign: "middle",
  })
}

function addHeader(slide, index, title, subtitle, options = {}) {
  const bg = options.bg ?? C.base
  addChip(
    slide,
    String(index).padStart(2, "0"),
    { left: 72, top: 54, width: 54, height: 30 },
    { fill: options.tagFill ?? C.paleOrange, color: options.tagColor ?? C.primary, fontSize: 15 },
  )
  addText(slide, title, { left: 72, top: 102, width: 760, height: 58 }, {
    fill: bg,
    color: options.titleColor ?? C.ink,
    fontSize: options.titleSize ?? 34,
    bold: true,
  })
  if (subtitle) {
    addText(slide, subtitle, { left: 72, top: 154, width: 760, height: 56 }, {
      fill: bg,
      color: options.subtitleColor ?? C.muted,
      fontSize: 17,
    })
  }
}

async function addImage(slide, assetPath, frame, options = {}) {
  const image = slide.images.add({
    dataUrl: await getImageDataUrl(assetPath),
    alt: options.alt ?? path.basename(assetPath),
    fit: options.fit ?? "cover",
  })
  image.frame = frame
  if (options.geometry) {
    image.geometry = options.geometry
  }
  if (options.crop) {
    image.crop = options.crop
  }
  return image
}

async function buildSlide1(slide) {
  setBackground(slide, C.base)
  addSoftBlob(slide, { left: -170, top: -110, width: 540, height: 540 }, C.paleOrange)
  addSoftBlob(slide, { left: 930, top: 430, width: 360, height: 360 }, C.blueSoft)

  await addImage(slide, ASSETS.logo, { left: 72, top: 38, width: 168, height: 40 }, {
    fit: "contain",
    alt: "Octopus logo",
  })

  addChip(slide, "数字员工操作系统 Agent OS", { left: 72, top: 98, width: 214, height: 32 }, {
    fill: C.surface,
    color: C.primary,
  })

  addText(slide, "Octopus\n产品介绍", { left: 72, top: 150, width: 420, height: 118 }, {
    fill: C.base,
    color: C.ink,
    fontSize: 44,
    bold: true,
  })

  addText(slide, "把 AI 对话窗口\n变成高效的交付流水线", { left: 72, top: 278, width: 470, height: 110 }, {
    fill: C.base,
    color: C.primary,
    fontSize: 28,
    bold: true,
  })

  addText(
    slide,
    "一个任务，一个智能体。描述你要交付的内容，Octopus 自动关联上下文，让 AI 自动调研、起草、对账、写代码。",
    { left: 72, top: 406, width: 420, height: 86 },
    { fill: C.base, color: C.muted, fontSize: 18 },
  )

  addChip(slide, "1 任务 = 1 智能体", { left: 72, top: 520, width: 144, height: 38 }, {
    fill: C.surface,
    color: C.ink,
  })
  addChip(slide, "100% 本地运行", { left: 228, top: 520, width: 132, height: 38 }, {
    fill: C.surface,
    color: C.ink,
  })
  addChip(slide, "任务进，结果出", { left: 372, top: 520, width: 132, height: 38 }, {
    fill: C.surface,
    color: C.ink,
  })

  addCard(slide, { left: 590, top: 92, width: 620, height: 510 }, {
    fill: C.surface,
    line: true,
    lineColor: C.border,
  })
  await addImage(slide, ASSETS.home, { left: 610, top: 112, width: 580, height: 470 }, {
    geometry: "roundRect",
    fit: "cover",
    alt: "Octopus website hero",
  })

  addText(slide, "100%\n本地运行", { left: 1040, top: 76, width: 136, height: 86 }, {
    geometry: "roundRect",
    fill: C.surface,
    color: C.primary,
    fontSize: 24,
    bold: true,
    align: "center",
    valign: "middle",
  })

  addText(slide, "全量上下文\n持续交付", { left: 570, top: 520, width: 164, height: 80 }, {
    geometry: "roundRect",
    fill: C.surface,
    color: C.ink,
    fontSize: 20,
    bold: true,
    align: "center",
    valign: "middle",
  })
}

async function buildSlide2(slide) {
  setBackground(slide, C.base)
  addHeader(
    slide,
    2,
    "它不是聊天窗口，而是数字员工操作系统",
    "Octopus 把一次性问答，变成持久化、可追踪、可审批的任务执行系统。",
  )

  addText(slide, "普通聊天式 AI", { left: 72, top: 224, width: 340, height: 34 }, {
    fill: C.base,
    color: C.ink,
    fontSize: 18,
    bold: true,
  })
  addCard(slide, { left: 72, top: 266, width: 340, height: 150 }, {
    fill: "#F6F1EA",
    line: true,
    lineColor: C.border,
  })
  addText(slide, "只停留在对话\n上下文很难持续继承\n结果难追踪，也难审批", { left: 96, top: 300, width: 292, height: 92 }, {
    fill: "#F6F1EA",
    color: C.muted,
    fontSize: 21,
    bold: false,
  })

  addText(slide, "Octopus", { left: 72, top: 440, width: 340, height: 34 }, {
    fill: C.base,
    color: C.primary,
    fontSize: 18,
    bold: true,
  })
  addCard(slide, { left: 72, top: 482, width: 340, height: 170 }, {
    fill: C.warm,
    line: true,
    lineColor: C.secondary,
  })
  addText(slide, "每个任务都有持久化智能体\n上下文、进度、执行链路持续保留\n可以暂停、恢复、审核与交付", { left: 96, top: 516, width: 292, height: 112 }, {
    fill: C.warm,
    color: C.ink,
    fontSize: 21,
    bold: true,
  })

  addCard(slide, { left: 454, top: 224, width: 754, height: 358 }, {
    fill: C.surface,
    line: true,
    lineColor: C.border,
  })
  await addImage(slide, ASSETS.agent, { left: 472, top: 242, width: 718, height: 322 }, {
    geometry: "roundRect",
    fit: "cover",
    alt: "Agent center screenshot",
  })

  addChip(slide, "持久化上下文", { left: 454, top: 614, width: 156, height: 40 }, {
    fill: C.surface,
    color: C.ink,
  })
  addChip(slide, "可追踪执行链路", { left: 626, top: 614, width: 170, height: 40 }, {
    fill: C.surface,
    color: C.ink,
  })
  addChip(slide, "可审核与审批", { left: 812, top: 614, width: 150, height: 40 }, {
    fill: C.surface,
    color: C.ink,
  })
  addChip(slide, "多任务并行", { left: 978, top: 614, width: 132, height: 40 }, {
    fill: C.surface,
    color: C.ink,
  })
}

async function buildSlide3(slide) {
  setBackground(slide, C.base)
  addHeader(
    slide,
    3,
    "工作方式",
    "从一句任务描述开始，Octopus 自动把上下文、执行和交付串成闭环。",
  )

  const steps = [
    ["01", "描述交付目标", "活动方案、报告、功能、审计，都从任务开始。"],
    ["02", "自动关联上下文", "项目文件、历史工作、团队规范自动进入任务上下文。"],
    ["03", "派发自主智能体", "每个任务获得独立智能体，持续执行而不是一次性回复。"],
    ["04", "监控与交付沉淀", "过程可追踪，可审批，结果回流到知识与项目资产里。"],
  ]
  const stepLefts = [72, 359, 646, 933]

  steps.forEach(([number, title, body], idx) => {
    addCard(slide, { left: stepLefts[idx], top: 224, width: 250, height: 130 }, {
      fill: idx === 2 ? C.warm : C.surface,
      line: true,
      lineColor: idx === 2 ? C.secondary : C.border,
    })
    addText(slide, number, { left: stepLefts[idx] + 22, top: 244, width: 40, height: 30 }, {
      fill: idx === 2 ? C.warm : C.surface,
      color: C.primary,
      fontSize: 18,
      bold: true,
    })
    addText(slide, title, { left: stepLefts[idx] + 22, top: 270, width: 186, height: 28 }, {
      fill: idx === 2 ? C.warm : C.surface,
      color: C.ink,
      fontSize: 20,
      bold: true,
    })
    addText(slide, body, { left: stepLefts[idx] + 22, top: 306, width: 206, height: 40 }, {
      fill: idx === 2 ? C.warm : C.surface,
      color: C.muted,
      fontSize: 14,
    })
  })

  addCard(slide, { left: 72, top: 392, width: 744, height: 264 }, {
    fill: C.surface,
    line: true,
    lineColor: C.border,
  })
  await addImage(slide, ASSETS.conversation, { left: 92, top: 412, width: 704, height: 224 }, {
    geometry: "roundRect",
    fit: "cover",
    alt: "Conversation screenshot",
  })
  addChip(slide, "持续会话 + 任务上下文", { left: 92, top: 428, width: 180, height: 32 }, {
    fill: C.surface,
    color: C.primary,
    fontSize: 13,
  })

  const proofCards = [
    ["持续会话", "任务不是一次性提问。会话状态、进度和下一步动作持续保留。", C.warm, C.secondary],
    ["上下文继承", "项目文件、历史工作与团队规范自动跟随任务进入执行链路。", C.surface, C.border],
    ["结果沉淀", "审批记录、交付物与知识资产在同一条链路里回流与复用。", C.blueCard, C.blueSoft],
  ]
  const proofTops = [392, 484, 576]

  proofCards.forEach(([title, body, fill, lineColor], idx) => {
    addCard(slide, { left: 848, top: proofTops[idx], width: 360, height: 72 }, {
      fill,
      line: true,
      lineColor,
    })
    addText(slide, title, { left: 870, top: proofTops[idx] + 12, width: 104, height: 22 }, {
      fill,
      color: title === "持续会话" ? C.primary : C.ink,
      fontSize: 17,
      bold: true,
    })
    addText(slide, body, { left: 980, top: proofTops[idx] + 12, width: 206, height: 38 }, {
      fill,
      color: C.muted,
      fontSize: 13,
    })
  })
}

async function buildSlide4(slide) {
  setBackground(slide, C.base)
  addHeader(
    slide,
    4,
    "真实界面，不是概念演示",
    "产品已经覆盖任务入口、指挥中心、数字员工和工具工作台。",
  )

  const imageFrames = [
    { left: 72, top: 228, width: 356, height: 196, asset: ASSETS.conversation, label: "对话与任务入口" },
    { left: 446, top: 228, width: 356, height: 196, asset: ASSETS.dashboard, label: "项目仪表盘" },
    { left: 72, top: 446, width: 356, height: 196, asset: ASSETS.agent, label: "数字员工中心" },
    { left: 446, top: 446, width: 356, height: 196, asset: ASSETS.builtin, label: "工具与 MCP" },
  ]

  for (const item of imageFrames) {
    addCard(slide, { left: item.left, top: item.top, width: item.width, height: item.height }, {
      fill: C.surface,
      line: true,
      lineColor: C.border,
    })
    await addImage(slide, item.asset, {
      left: item.left + 12,
      top: item.top + 12,
      width: item.width - 24,
      height: item.height - 44,
    }, {
      geometry: "roundRect",
      fit: "cover",
      alt: item.label,
    })
    addChip(slide, item.label, {
      left: item.left + 12,
      top: item.top + item.height - 28,
      width: 138,
      height: 22,
    }, {
      fill: C.warm,
      color: C.ink,
      fontSize: 12,
    })
  }

  const rightCards = [
    ["对话入口", "用户一句话下任务，任务目标、上下文、下一步动作都在同一界面推进。"],
    ["状态监控", "项目仪表盘让会话、资源、知识、Agent 与动态一屏可见。"],
    ["数字员工", "按角色建立 Agent 资源池，支持财务、研发、运营等岗位化配置。"],
    ["工具工作台", "统一管理内置工具、技能目录与 MCP 服务，给智能体接上真实系统。"],
  ]
  const rightTops = [232, 334, 436, 538]

  rightCards.forEach(([title, body], idx) => {
    addCard(slide, { left: 848, top: rightTops[idx], width: 360, height: 84 }, {
      fill: idx % 2 === 0 ? C.surface : C.blueCard,
      line: true,
      lineColor: idx % 2 === 0 ? C.border : C.blueSoft,
    })
    addText(slide, title, { left: 872, top: rightTops[idx] + 14, width: 150, height: 22 }, {
      fill: idx % 2 === 0 ? C.surface : C.blueCard,
      color: C.ink,
      fontSize: 18,
      bold: true,
    })
    addText(slide, body, { left: 872, top: rightTops[idx] + 38, width: 304, height: 34 }, {
      fill: idx % 2 === 0 ? C.surface : C.blueCard,
      color: C.muted,
      fontSize: 13,
    })
  })

  addText(
    slide,
    "同一套桌面工作台，把对话、工具、知识和治理放进同一个运行面板里。",
    { left: 848, top: 640, width: 330, height: 36 },
    { fill: C.base, color: C.primary, fontSize: 16, bold: true },
  )
}

async function buildSlide5(slide) {
  setBackground(slide, C.base)
  addHeader(
    slide,
    5,
    "平台能力与治理边界",
    "它既能干活，也知道边界在哪里。",
  )

  addCard(slide, { left: 72, top: 224, width: 500, height: 396 }, {
    fill: C.surface,
    line: true,
    lineColor: C.border,
  })
  await addImage(slide, ASSETS.rbac, { left: 90, top: 242, width: 464, height: 360 }, {
    geometry: "roundRect",
    fit: "cover",
    alt: "RBAC screenshot",
  })

  const topChips = [
    ["100% 本地", 620, 224, 118],
    ["权限控制", 748, 224, 118],
    ["审计回放", 876, 224, 118],
    ["信创适配", 1004, 224, 118],
  ]
  topChips.forEach(([text, left, top, width]) => {
    addChip(slide, text, { left, top, width, height: 34 }, {
      fill: C.surface,
      color: C.ink,
      fontSize: 14,
    })
  })

  const cards = [
    ["工具与系统接入", "通过标准协议接入数据库、内部系统和外部工具。"],
    ["浏览器自主执行", "内置浏览器与隔离环境，支持自动调研与执行。"],
    ["过程可回放", "执行链路、上下文与结果产物都可查看与复盘。"],
    ["角色权限可控", "角色、菜单、访问范围和治理策略都能落到系统层。"],
    ["离线与内网支持", "零云端依赖，数据不出域，适合离线和内网环境。"],
    ["信创基础适配", "适配国产 CPU、操作系统与数据库，满足合规基础设施要求。"],
  ]
  const cardFrames = [
    { left: 620, top: 276 },
    { left: 908, top: 276 },
    { left: 620, top: 404 },
    { left: 908, top: 404 },
    { left: 620, top: 532 },
    { left: 908, top: 532 },
  ]

  cards.forEach(([title, body], idx) => {
    const fill = idx % 2 === 0 ? C.surface : C.blueCard
    addCard(slide, { left: cardFrames[idx].left, top: cardFrames[idx].top, width: 252, height: 104 }, {
      fill,
      line: true,
      lineColor: idx % 2 === 0 ? C.border : C.blueSoft,
    })
    addText(slide, title, { left: cardFrames[idx].left + 18, top: cardFrames[idx].top + 14, width: 216, height: 22 }, {
      fill,
      color: C.ink,
      fontSize: 16,
      bold: true,
    })
    addText(slide, body, { left: cardFrames[idx].left + 18, top: cardFrames[idx].top + 40, width: 216, height: 44 }, {
      fill,
      color: C.muted,
      fontSize: 13,
    })
  })
}

async function buildSlide6(slide) {
  setBackground(slide, C.base)
  addHeader(
    slide,
    6,
    "它能帮哪些团队",
    "从个人到企业，从市场到财务，Octopus 覆盖真实工作链路。",
  )

  const segments = [
    ["个人", "调研、写作、代码、知识管理，一个人也能并行推进多个任务。"],
    ["团队", "围绕任务派发多个智能体，与人类成员并行协作。"],
    ["企业", "私有化部署、权限治理、审计日志与信创底座一起落地。"],
  ]
  const segmentFills = [C.surface, C.warm, C.blueCard]
  const segmentTops = [228, 384, 540]

  segments.forEach(([title, body], idx) => {
    addCard(slide, { left: 72, top: segmentTops[idx], width: 276, height: 124 }, {
      fill: segmentFills[idx],
      line: true,
      lineColor: idx === 0 ? C.border : idx === 1 ? C.secondary : C.blueSoft,
    })
    addText(slide, title, { left: 96, top: segmentTops[idx] + 18, width: 120, height: 24 }, {
      fill: segmentFills[idx],
      color: idx === 1 ? C.primary : C.ink,
      fontSize: 22,
      bold: true,
    })
    addText(slide, body, { left: 96, top: segmentTops[idx] + 52, width: 228, height: 48 }, {
      fill: segmentFills[idx],
      color: C.muted,
      fontSize: 14,
    })
  })

  const cases = [
    ["市场", "活动发布自动驾驶", "竞品分析、落地页文案、社媒素材包"],
    ["销售", "客户情报分析", "EMEA SaaS 调研与个性化触达邮件"],
    ["研发", "并行功能交付", "组件、接口、测试与 PR 摘要并行推进"],
    ["内容运营", "跨平台发布", "Changelog 同步到博客、Twitter、LinkedIn"],
    ["客户成功", "工单智能分拣", "分类、起草回复、升级紧急问题"],
    ["财务", "董事会级报告", "收入趋势、燃烧率、Runway 预测"],
  ]
  const casePositions = [
    { left: 390, top: 228 },
    { left: 794, top: 228 },
    { left: 390, top: 372 },
    { left: 794, top: 372 },
    { left: 390, top: 516 },
    { left: 794, top: 516 },
  ]

  cases.forEach(([category, title, body], idx) => {
    addCard(slide, { left: casePositions[idx].left, top: casePositions[idx].top, width: 340, height: 118 }, {
      fill: C.surface,
      line: true,
      lineColor: C.border,
    })
    addChip(slide, category, {
      left: casePositions[idx].left + 18,
      top: casePositions[idx].top + 14,
      width: 78,
      height: 24,
    }, {
      fill: C.paleOrange,
      color: C.primary,
      fontSize: 12,
    })
    addText(slide, title, { left: casePositions[idx].left + 18, top: casePositions[idx].top + 44, width: 240, height: 22 }, {
      fill: C.surface,
      color: C.ink,
      fontSize: 18,
      bold: true,
    })
    addText(slide, body, { left: casePositions[idx].left + 18, top: casePositions[idx].top + 72, width: 292, height: 30 }, {
      fill: C.surface,
      color: C.muted,
      fontSize: 13,
    })
  })
}

async function buildSlide7(slide) {
  setBackground(slide, C.dark)
  addSoftBlob(slide, { left: -180, top: -120, width: 520, height: 520 }, C.darkAlt)
  addSoftBlob(slide, { left: 910, top: 440, width: 340, height: 340 }, "#3A2413")

  await addImage(slide, ASSETS.logo, { left: 72, top: 44, width: 160, height: 38 }, {
    fit: "contain",
    alt: "Octopus logo",
  })

  addText(slide, "把 AI 从零散工具，升级为正式生产体系", { left: 72, top: 114, width: 670, height: 60 }, {
    fill: C.dark,
    color: C.darkText,
    fontSize: 34,
    bold: true,
  })
  addText(
    slide,
    "Octopus 的价值不只是提升单点效率，而是把执行、治理和沉淀放进同一平台。",
    { left: 72, top: 184, width: 620, height: 48 },
    { fill: C.dark, color: C.darkMuted, fontSize: 17 },
  )

  const values = [
    ["01", "降本增效", "把复杂、多角色、长流程任务交给可持续运行的智能体体系推进。"],
    ["02", "统一治理", "权限、审批、预算与审计进入系统层，而不是停留在演示概念。"],
    ["03", "平台升级", "让执行数据、成果物和知识资产持续沉淀复用，而不是留在单次对话里。"],
  ]
  const lefts = [72, 454, 836]

  values.forEach(([num, title, body], idx) => {
    addCard(slide, { left: lefts[idx], top: 286, width: 338, height: 194 }, {
      fill: C.surface,
      line: false,
    })
    addText(slide, num, { left: lefts[idx] + 18, top: 304, width: 52, height: 30 }, {
      fill: C.surface,
      color: C.primary,
      fontSize: 20,
      bold: true,
      align: "center",
      valign: "middle",
    })
    addText(slide, title, { left: lefts[idx] + 22, top: 342, width: 180, height: 28 }, {
      fill: C.surface,
      color: C.ink,
      fontSize: 24,
      bold: true,
    })
    addText(slide, body, { left: lefts[idx] + 22, top: 384, width: 286, height: 56 }, {
      fill: C.surface,
      color: C.muted,
      fontSize: 15,
    })
  })

  addText(slide, "任务进，结果出", { left: 72, top: 564, width: 240, height: 34 }, {
    fill: C.dark,
    color: C.primary,
    fontSize: 26,
    bold: true,
  })
  addText(
    slide,
    "一个任务，一个智能体。让 AI 真正进入正式经营体系。",
    { left: 72, top: 610, width: 430, height: 34 },
    { fill: C.dark, color: C.darkMuted, fontSize: 15 },
  )

  addCard(slide, { left: 850, top: 536, width: 358, height: 122 }, {
    fill: C.surface,
    line: false,
  })
  await addImage(slide, ASSETS.dashboard, { left: 864, top: 550, width: 330, height: 94 }, {
    geometry: "roundRect",
    fit: "cover",
    alt: "Dashboard closing proof",
  })
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
