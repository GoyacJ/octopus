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
  contentCreationLens: path.join(OUT_DIR, "assets", "scenarios", "content-creation-lens.jpg"),
  officeWritingDesk: path.join(OUT_DIR, "assets", "scenarios", "office-writing-desk.jpg"),
  softwareDevCode: path.join(OUT_DIR, "assets", "scenarios", "software-dev-code.jpg"),
  financeOpsCalculator: path.join(OUT_DIR, "assets", "scenarios", "finance-ops-calculator.jpg"),
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
  await buildSlide8(presentation.slides.add())

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
    const extension = path.extname(filePath).toLowerCase()
    const mimeType = {
      ".png": "image/png",
      ".jpg": "image/jpeg",
      ".jpeg": "image/jpeg",
      ".webp": "image/webp",
    }[extension] ?? "application/octet-stream"
    const encoded = `data:${mimeType};base64,${data.toString("base64")}`
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
  addSoftBlob(slide, { left: -120, top: 480, width: 320, height: 320 }, C.paleOrange)
  addSoftBlob(slide, { left: 1020, top: 180, width: 260, height: 260 }, C.blueSoft)
  addHeader(
    slide,
    3,
    "生而不同",
    "不是把聊天做得更顺，而是把交付系统从一开始就按任务驱动设计。",
  )

  addCard(slide, { left: 72, top: 224, width: 1136, height: 92 }, {
    fill: C.surface,
    line: true,
    lineColor: C.border,
  })
  addText(slide, "每个团队都有待交付的工作。Octopus 为每个任务分配自主 AI 智能体。", { left: 96, top: 244, width: 632, height: 24 }, {
    fill: C.surface,
    color: C.ink,
    fontSize: 19,
    bold: true,
  })
  addText(slide, "全量上下文，全程掌控，所有部门适用。免费开源，Apache 2.0。", { left: 96, top: 276, width: 620, height: 22 }, {
    fill: C.surface,
    color: C.muted,
    fontSize: 15,
  })

  addChip(slide, "开源", { left: 826, top: 246, width: 82, height: 30 }, {
    fill: C.warm,
    color: C.primary,
    fontSize: 13,
  })
  addChip(slide, "Apache 2.0", { left: 918, top: 246, width: 142, height: 30 }, {
    fill: C.surface,
    color: C.ink,
    fontSize: 13,
  })
  addChip(slide, "免费使用", { left: 1070, top: 246, width: 106, height: 30 }, {
    fill: C.blueCard,
    color: C.ink,
    fontSize: 13,
  })

  const principles = [
    ["01", "1:1 任务 → 智能体", "一个任务，一个智能体。完整上下文，完整可追溯，零歧义。", C.surface, C.border],
    ["02", "100% 本地 & 隐私", "一切运行在你的机器上。零遥测、零云端依赖，数据绝不外传。", C.warm, C.secondary],
    ["03", "15+ 并行智能体", "同时交付 15+ 个任务。暂停和恢复任何智能体，上下文不丢失。", C.blueCard, C.blueSoft],
    ["04", "4-Layer 深度上下文引擎", "组织知识、项目上下文、团队标准和任务级指令一起进入执行。", C.surface, C.border],
  ]
  const principleFrames = [
    { left: 72, top: 350, width: 544, height: 132 },
    { left: 664, top: 350, width: 544, height: 132 },
    { left: 72, top: 506, width: 544, height: 132 },
    { left: 664, top: 506, width: 544, height: 132 },
  ]

  principles.forEach(([number, title, body, fill, lineColor], idx) => {
    const frame = principleFrames[idx]
    addCard(slide, frame, {
      fill,
      line: true,
      lineColor,
    })
    addText(slide, number, { left: frame.left + 20, top: frame.top + 16, width: 40, height: 24 }, {
      fill,
      color: C.primary,
      fontSize: 18,
      bold: true,
    })
    addText(slide, title, { left: frame.left + 20, top: frame.top + 46, width: 380, height: 26 }, {
      fill,
      color: C.ink,
      fontSize: title === "4-Layer 深度上下文引擎" ? 18 : 20,
      bold: true,
    })
    addText(slide, body, { left: frame.left + 20, top: frame.top + 82, width: 486, height: 34 }, {
      fill,
      color: C.muted,
      fontSize: 14,
    })
  })
}

async function buildSlide4(slide) {
  setBackground(slide, C.base)
  addHeader(
    slide,
    4,
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

async function buildSlide5(slide) {
  setBackground(slide, C.base)
  addHeader(
    slide,
    5,
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
    ["任务到智能体流水线", "内置任务追踪器，每个任务自动成为自主智能体。任务清单就是交付计划。"],
    ["智能体能力", "技能库、团队规范、集成能力一起装备到智能体，确保理解业务并遵循流程。"],
    ["智能体指挥中心", "实时看到当前任务、进度、操作审核与交付审批，一个屏幕全局可见。"],
    ["随时随地工作", "通过 IM、调度和 Webhook 派发智能体与接收通知，重复工作可自动运行。"],
  ]
  const rightTops = [232, 334, 436, 538]

  rightCards.forEach(([title, body], idx) => {
    addCard(slide, { left: 848, top: rightTops[idx], width: 360, height: 84 }, {
      fill: idx % 2 === 0 ? C.surface : C.blueCard,
      line: true,
      lineColor: idx % 2 === 0 ? C.border : C.blueSoft,
    })
    addText(slide, title, { left: 872, top: rightTops[idx] + 12, width: 226, height: 22 }, {
      fill: idx % 2 === 0 ? C.surface : C.blueCard,
      color: C.ink,
      fontSize: 17,
      bold: true,
    })
    addText(slide, body, { left: 872, top: rightTops[idx] + 36, width: 304, height: 36 }, {
      fill: idx % 2 === 0 ? C.surface : C.blueCard,
      color: C.muted,
      fontSize: 12,
    })
  })

  addText(
    slide,
    "平台全貌不是单个页面，而是一套把任务、能力、监控和通知串起来的运行系统。",
    { left: 848, top: 640, width: 332, height: 36 },
    { fill: C.base, color: C.primary, fontSize: 16, bold: true },
  )
}

async function buildSlide6(slide) {
  setBackground(slide, C.base)
  addHeader(
    slide,
    6,
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

async function buildSlide7(slide) {
  setBackground(slide, C.base)
  addSoftBlob(slide, { left: -120, top: 486, width: 280, height: 280 }, C.paleOrange)
  addSoftBlob(slide, { left: 1010, top: 196, width: 250, height: 250 }, C.blueSoft)
  addHeader(
    slide,
    7,
    "它能帮哪些场景",
    "不只会聊天。Octopus 能直接接入真实工作链路，覆盖内容创作、办公写作、软件研发和财务运营。",
  )

  const scenarios = [
    {
      title: "内容创作",
      summary: "短剧、漫剧、视觉素材和脚本节奏\n都能拆成连续任务链。",
      note: "从灵感到交付，始终保留同一份上下文。",
      tags: [
        ["脚本拆解", 84],
        ["素材整理", 84],
      ],
      image: ASSETS.contentCreationLens,
      fill: C.surface,
      lineColor: C.border,
      accent: C.primary,
      tagFill: C.paleOrange,
      tagColor: C.primary,
      mediaFill: "#F8F0E7",
    },
    {
      title: "办公写作",
      summary: "会议纪要、方案初稿、周报月报\n和知识归档在同一上下文里完成。",
      note: "写作不再回到空白页。",
      tags: [
        ["结构化写作", 92],
        ["知识沉淀", 84],
      ],
      image: ASSETS.officeWritingDesk,
      fill: "#FCFAF7",
      lineColor: "#E9DED2",
      accent: "#8B6C53",
      tagFill: "#F3ECE5",
      tagColor: "#8B6C53",
      mediaFill: "#F6EFE8",
    },
    {
      title: "软件研发",
      summary: "需求拆解、代码实现、测试补齐\n和 PR 摘要持续推进。",
      note: "从任务到提交，不再回到零上下文。",
      tags: [
        ["代码生成", 84],
        ["测试补齐", 84],
      ],
      image: ASSETS.softwareDevCode,
      fill: "#F7FAFE",
      lineColor: C.blueSoft,
      accent: "#3E648E",
      tagFill: "#DCE9F8",
      tagColor: "#355C8C",
      mediaFill: "#EAF2FB",
    },
    {
      title: "财务运营",
      summary: "日报汇总、经营分析、预算对账\n与异常提醒形成稳定运营节奏。",
      note: "报表、对账和提醒可以按周期自动执行。",
      tags: [
        ["报表生成", 84],
        ["异常提醒", 84],
      ],
      image: ASSETS.financeOpsCalculator,
      fill: "#F7FAF2",
      lineColor: "#D6E4D1",
      accent: "#73875C",
      tagFill: "#DCE8D8",
      tagColor: "#617349",
      mediaFill: "#EAF3E8",
    },
  ]
  const cardFrames = [
    { left: 72, top: 228, width: 552, height: 192 },
    { left: 656, top: 228, width: 552, height: 192 },
    { left: 72, top: 444, width: 552, height: 192 },
    { left: 656, top: 444, width: 552, height: 192 },
  ]

  for (const [idx, scenario] of scenarios.entries()) {
    const frame = cardFrames[idx]
    addCard(slide, frame, {
      fill: scenario.fill,
      line: true,
      lineColor: scenario.lineColor,
    })

    const accent = slide.shapes.add({ geometry: "roundRect" })
    accent.frame = { left: frame.left + 20, top: frame.top + 18, width: 42, height: 6 }
    accent.fill.color = scenario.accent
    accent.line.visible = false

    addText(slide, scenario.title, { left: frame.left + 20, top: frame.top + 36, width: 320, height: 28 }, {
      fill: scenario.fill,
      color: C.ink,
      fontSize: 24,
      bold: true,
    })
    addText(slide, scenario.summary, { left: frame.left + 20, top: frame.top + 80, width: 320, height: 48 }, {
      fill: scenario.fill,
      color: C.ink,
      fontSize: 15,
    })
    addText(slide, scenario.note, { left: frame.left + 20, top: frame.top + 132, width: 320, height: 20 }, {
      fill: scenario.fill,
      color: C.muted,
      fontSize: 13,
    })

    let tagLeft = frame.left + 20
    for (const [label, width] of scenario.tags) {
      addChip(slide, label, { left: tagLeft, top: frame.top + 156, width, height: 24 }, {
        fill: scenario.tagFill,
        color: scenario.tagColor,
        fontSize: 12,
      })
      tagLeft += width + 8
    }

    addCard(slide, { left: frame.left + 376, top: frame.top + 16, width: 156, height: 160 }, {
      fill: scenario.mediaFill,
      line: false,
    })
    await addImage(slide, scenario.image, { left: frame.left + 384, top: frame.top + 24, width: 140, height: 144 }, {
      geometry: "roundRect",
      fit: "cover",
      alt: `${scenario.title} scene`,
    })
  }
}

async function buildSlide8(slide) {
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
  addText(
    slide,
    "免费开源 · Apache 2.0 · 100% 本地",
    { left: 72, top: 646, width: 360, height: 22 },
    { fill: C.dark, color: C.darkMuted, fontSize: 13, bold: true },
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
