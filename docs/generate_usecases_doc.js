const { Document, Packer, Paragraph, TextRun, Table, TableRow, TableCell,
        Header, Footer, AlignmentType, HeadingLevel, BorderStyle, WidthType,
        ShadingType, VerticalAlign, PageNumber, PageBreak, LevelFormat } = require('docx');
const fs = require('fs');

// 边框样式
const border = { style: BorderStyle.SINGLE, size: 1, color: "CCCCCC" };
const borders = { top: border, bottom: border, left: border, right: border };

// 表格单元格辅助函数
function createTableCell(text, width, isHeader = false, align = AlignmentType.LEFT) {
    return new TableCell({
        borders,
        width: { size: width, type: WidthType.DXA },
        shading: isHeader ? { fill: "2E75B6", type: ShadingType.CLEAR } : { fill: "FFFFFF", type: ShadingType.CLEAR },
        margins: { top: 80, bottom: 80, left: 120, right: 120 },
        verticalAlign: VerticalAlign.CENTER,
        children: [new Paragraph({
            alignment: align,
            children: [new TextRun({
                text: text,
                font: "Arial",
                size: isHeader ? 22 : 20,
                bold: isHeader,
                color: isHeader ? "FFFFFF" : "000000"
            })]
        })]
    });
}

function createCell(text, width, fill = "FFFFFF", bold = false, color = "000000") {
    return new TableCell({
        borders,
        width: { size: width, type: WidthType.DXA },
        shading: { fill: fill, type: ShadingType.CLEAR },
        margins: { top: 80, bottom: 80, left: 120, right: 120 },
        children: [new Paragraph({
            children: [new TextRun({ text: text, font: "Arial", size: 20, bold: bold, color: color })]
        })]
    });
}

// 创建文档
const doc = new Document({
    styles: {
        default: { document: { run: { font: "Arial", size: 24 } } },
        paragraphStyles: [
            {
                id: "Heading1", name: "Heading 1", basedOn: "Normal", next: "Normal", quickFormat: true,
                run: { size: 36, bold: true, font: "Arial", color: "2E75B6" },
                paragraph: { spacing: { before: 360, after: 240 }, outlineLevel: 0 }
            },
            {
                id: "Heading2", name: "Heading 2", basedOn: "Normal", next: "Normal", quickFormat: true,
                run: { size: 28, bold: true, font: "Arial", color: "2E75B6" },
                paragraph: { spacing: { before: 300, after: 180 }, outlineLevel: 1 }
            },
            {
                id: "Heading3", name: "Heading 3", basedOn: "Normal", next: "Normal", quickFormat: true,
                run: { size: 24, bold: true, font: "Arial", color: "404040" },
                paragraph: { spacing: { before: 240, after: 120 }, outlineLevel: 2 }
            },
        ]
    },
    numbering: {
        config: [
            {
                reference: "bullets",
                levels: [{
                    level: 0, format: LevelFormat.BULLET, text: "\u2022", alignment: AlignmentType.LEFT,
                    style: { paragraph: { indent: { left: 720, hanging: 360 } } }
                }]
            },
            {
                reference: "numbers",
                levels: [{
                    level: 0, format: LevelFormat.DECIMAL, text: "%1.", alignment: AlignmentType.LEFT,
                    style: { paragraph: { indent: { left: 720, hanging: 360 } } }
                }]
            }
        ]
    },
    sections: [{
        properties: {
            page: {
                size: { width: 11906, height: 16838 }, // A4
                margin: { top: 1440, right: 1440, bottom: 1440, left: 1440 }
            }
        },
        headers: {
            default: new Header({
                children: [new Paragraph({
                    alignment: AlignmentType.RIGHT,
                    border: { bottom: { style: BorderStyle.SINGLE, size: 6, color: "2E75B6", space: 1 } },
                    children: [new TextRun({ text: "GyID 应用场景分析", font: "Arial", size: 20, color: "666666" })]
                })]
            })
        },
        footers: {
            default: new Footer({
                children: [new Paragraph({
                    alignment: AlignmentType.CENTER,
                    border: { top: { style: BorderStyle.SINGLE, size: 6, color: "2E75B6", space: 1 } },
                    children: [
                        new TextRun({ text: "第 ", font: "Arial", size: 20, color: "666666" }),
                        new TextRun({ children: [PageNumber.CURRENT], font: "Arial", size: 20, color: "666666" }),
                        new TextRun({ text: " 页", font: "Arial", size: 20, color: "666666" })
                    ]
                })]
            })
        },
        children: [
            // 标题
            new Paragraph({
                alignment: AlignmentType.CENTER,
                spacing: { after: 480 },
                children: [new TextRun({ text: "GyID 应用场景分析", font: "Arial", size: 56, bold: true, color: "2E75B6" })]
            }),
            new Paragraph({
                alignment: AlignmentType.CENTER,
                spacing: { after: 600 },
                children: [new TextRun({ text: "去中心化多维度动态身份系统", font: "Arial", size: 28, color: "666666" })]
            }),

            // 项目现状总结
            new Paragraph({ heading: HeadingLevel.HEADING_1, children: [new TextRun("项目现状总结")] }),

            new Table({
                width: { size: 9026, type: WidthType.DXA },
                columnWidths: [2000, 4500, 2526],
                rows: [
                    new TableRow({ children: [
                        createTableCell("模块", 2000, true),
                        createTableCell("功能", 4500, true),
                        createTableCell("成熟度", 2526, true)
                    ]}),
                    new TableRow({ children: [
                        createCell("核心算法", 2000, "F0F8FF"),
                        createCell("四维度融合 (硬件+地理+时间+头像)", 4500),
                        createCell("完善", 2526, "E8F5E9", true, "2E7D32")
                    ]}),
                    new TableRow({ children: [
                        createCell("链上锚定", 2000, "F0F8FF"),
                        createCell("Polygon PoS + Aptos", 4500),
                        createCell("完善", 2526, "E8F5E9", true, "2E7D32")
                    ]}),
                    new TableRow({ children: [
                        createCell("多设备关联", 2000, "F0F8FF"),
                        createCell("主从设备树形结构", 4500),
                        createCell("完善", 2526, "E8F5E9", true, "2E7D32")
                    ]}),
                    new TableRow({ children: [
                        createCell("跨平台 SDK", 2000, "F0F8FF"),
                        createCell("Windows/Linux/macOS/Web/Android/iOS", 4500),
                        createCell("完善", 2526, "E8F5E9", true, "2E7D32")
                    ]}),
                    new TableRow({ children: [
                        createCell("GUI 应用", 2000, "F0F8FF"),
                        createCell("Dioxus 桌面界面", 4500),
                        createCell("开发中", 2526, "FFF3E0", true, "E65100")
                    ]}),
                    new TableRow({ children: [
                        createCell("API 层", 2000, "F0F8FF"),
                        createCell("统一 SDK 接口", 4500),
                        createCell("完善", 2526, "E8F5E9", true, "2E7D32")
                    ]})
                ]
            }),

            // 商业应用场景
            new Paragraph({ heading: HeadingLevel.HEADING_1, children: [new TextRun("商业应用场景")] }),

            // 1. 数字身份认证
            new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun("1. 数字身份认证（核心场景）")] }),

            new Paragraph({
                spacing: { after: 120 },
                children: [
                    new TextRun({ text: "用户痛点：", font: "Arial", size: 22, bold: true }),
                    new TextRun({ text: "传统用户名密码容易被盗、泄露、钓鱼", font: "Arial", size: 22 })
                ]
            }),
            new Paragraph({
                spacing: { after: 200 },
                children: [
                    new TextRun({ text: "GyID 方案：", font: "Arial", size: 22, bold: true }),
                    new TextRun({ text: "基于硬件指纹+地理位置的设备绑定身份", font: "Arial", size: 22 })
                ]
            }),

            new Table({
                width: { size: 9026, type: WidthType.DXA },
                columnWidths: [2500, 6526],
                rows: [
                    new TableRow({ children: [
                        createTableCell("应用", 2500, true),
                        createTableCell("说明", 6526, true)
                    ]}),
                    new TableRow({ children: [
                        createCell("无密码登录", 2500, "E3F2FD"),
                        createCell("用户设备就是身份，无需记忆密码", 6526)
                    ]}),
                    new TableRow({ children: [
                        createCell("设备信任", 2500, "E3F2FD"),
                        createCell("常用设备自动识别，可信地理位置白名单", 6526)
                    ]}),
                    new TableRow({ children: [
                        createCell("MFA 增强", 2500, "E3F2FD"),
                        createCell("作为多因素认证的硬件因子", 6526)
                    ]})
                ]
            }),

            new Paragraph({
                spacing: { before: 200 },
                children: [new TextRun({ text: "案例：", font: "Arial", size: 22, bold: true })]
            }),
            new Paragraph({ numbering: { reference: "bullets", level: 0 }, children: [new TextRun({ text: "企业内部系统登录", font: "Arial", size: 22 })] }),
            new Paragraph({ numbering: { reference: "bullets", level: 0 }, children: [new TextRun({ text: "银行 App 设备绑定", font: "Arial", size: 22 })] }),
            new Paragraph({ numbering: { reference: "bullets", level: 0 }, children: [new TextRun({ text: "政务服务平台身份认证", font: "Arial", size: 22 })] }),

            // 2. 防欺诈与风控
            new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun("2. 防欺诈与风控")] }),

            new Paragraph({
                spacing: { after: 200 },
                children: [
                    new TextRun({ text: "GyID 优势：", font: "Arial", size: 22, bold: true }),
                    new TextRun({ text: "多维度验证，难以伪造或模拟", font: "Arial", size: 22 })
                ]
            }),

            new Table({
                width: { size: 9026, type: WidthType.DXA },
                columnWidths: [2500, 6526],
                rows: [
                    new TableRow({ children: [
                        createTableCell("场景", 2500, true),
                        createTableCell("验证要素", 6526, true)
                    ]}),
                    new TableRow({ children: [
                        createCell("账号注册", 2500, "FFF8E1"),
                        createCell("硬件指纹 + 地理位置，防止多开", 6526)
                    ]}),
                    new TableRow({ children: [
                        createCell("异常登录", 2500, "FFF8E1"),
                        createCell("新设备触发告警", 6526)
                    ]}),
                    new TableRow({ children: [
                        createCell("薅羊毛防护", 2500, "FFF8E1"),
                        createCell("真实物理设备和位置绑定", 6526)
                    ]}),
                    new TableRow({ children: [
                        createCell("洗钱监控", 2500, "FFF8E1"),
                        createCell("地理位置与 IP 交叉验证", 6526)
                    ]})
                ]
            }),

            new Paragraph({
                spacing: { before: 200 },
                children: [new TextRun({ text: "技术优势：", font: "Arial", size: 22, bold: true })]
            }),
            new Paragraph({ numbering: { reference: "bullets", level: 0 }, children: [new TextRun({ text: "无法通过 VPN 伪造地理位置（GPS + WiFi BSSID 双重验证）", font: "Arial", size: 22 })] }),
            new Paragraph({ numbering: { reference: "bullets", level: 0 }, children: [new TextRun({ text: "无法通过虚拟机绕过硬件指纹（采集真实系统信息）", font: "Arial", size: 22 })] }),

            // 3. 个人数字资产管理
            new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun("3. 个人数字资产管理")] }),

            new Paragraph({
                spacing: { after: 200 },
                children: [new TextRun({ text: 'GyID 可作为数字资产的"身份证"', font: "Arial", size: 22 })]
            }),

            new Table({
                width: { size: 9026, type: WidthType.DXA },
                columnWidths: [3000, 6026],
                rows: [
                    new TableRow({ children: [
                        createTableCell("资产类型", 3000, true),
                        createTableCell("应用", 6026, true)
                    ]}),
                    new TableRow({ children: [
                        createCell("NFT", 3000, "E8F5E9"),
                        createCell("NFT 所有权与创作者身份绑定", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("游戏道具", 3000, "E8F5E9"),
                        createCell("游戏装备与玩家 GyID 永久绑定", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("数字藏品", 3000, "E8F5E9"),
                        createCell("藏品确权与流转记录", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("域名/ENS", 3000, "E8F5E9"),
                        createCell("Web3 身份与 GyID 关联", 6026)
                    ]})
                ]
            }),

            new Paragraph({
                spacing: { before: 200 },
                children: [new TextRun({ text: "链上锚定价值：", font: "Arial", size: 22, bold: true })]
            }),
            new Paragraph({ numbering: { reference: "bullets", level: 0 }, children: [new TextRun({ text: "GyID 哈希上链，永久可验证", font: "Arial", size: 22 })] }),
            new Paragraph({ numbering: { reference: "bullets", level: 0 }, children: [new TextRun({ text: "Polygon/Aptos 双链支持，高可用", font: "Arial", size: 22 })] }),

            // 分页
            new Paragraph({ children: [new PageBreak()] }),

            // 4. GEOYUAN APP 生态
            new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun("4. GEOYUAN APP 生态")] }),

            new Paragraph({
                spacing: { after: 200 },
                children: [new TextRun({ text: "根据项目规划，GyID 将作为 GEOYUAN APP 的核心身份系统：", font: "Arial", size: 22 })]
            }),

            new Table({
                width: { size: 9026, type: WidthType.DXA },
                columnWidths: [3000, 6026],
                rows: [
                    new TableRow({ children: [
                        createTableCell("功能", 3000, true),
                        createTableCell("实现", 6026, true)
                    ]}),
                    new TableRow({ children: [
                        createCell("登录凭证", 3000, "F3E5F5"),
                        createCell("GyID 替代传统账号密码", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("数字资产", 3000, "F3E5F5"),
                        createCell("APP 内资产与 GyID 绑定", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("社交身份", 3000, "F3E5F5"),
                        createCell("去中心化社交图谱", 6026)
                    ]})
                ]
            }),

            // 5. IoT 设备身份管理
            new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun("5. IoT 设备身份管理")] }),

            new Paragraph({
                spacing: { after: 200 },
                children: [new TextRun({ text: "GyID 可扩展用于物联网设备身份", font: "Arial", size: 22 })]
            }),

            new Table({
                width: { size: 9026, type: WidthType.DXA },
                columnWidths: [2500, 6526],
                rows: [
                    new TableRow({ children: [
                        createTableCell("场景", 2500, true),
                        createTableCell("方案", 6526, true)
                    ]}),
                    new TableRow({ children: [
                        createCell("智能家居", 2500, "E0F7FA"),
                        createCell("设备 GyID + 主人 GyID 授权", 6526)
                    ]}),
                    new TableRow({ children: [
                        createCell("车联网", 2500, "E0F7FA"),
                        createCell("车辆 GyID 身份与行驶记录关联", 6526)
                    ]}),
                    new TableRow({ children: [
                        createCell("工业设备", 2500, "E0F7FA"),
                        createCell("工厂设备身份认证与监控", 6526)
                    ]})
                ]
            }),

            new Paragraph({
                spacing: { before: 200 },
                children: [new TextRun({ text: "优势：", font: "Arial", size: 22, bold: true })]
            }),
            new Paragraph({ numbering: { reference: "bullets", level: 0 }, children: [new TextRun({ text: "去中心化，无需中心化 CA 机构", font: "Arial", size: 22 })] }),

            // 6. 企业安全与合规
            new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun("6. 企业安全与合规")] }),

            new Table({
                width: { size: 9026, type: WidthType.DXA },
                columnWidths: [3000, 6026],
                rows: [
                    new TableRow({ children: [
                        createTableCell("场景", 3000, true),
                        createTableCell("GyID 价值", 6026, true)
                    ]}),
                    new TableRow({ children: [
                        createCell("远程办公", 3000, "FBE9E7"),
                        createCell("设备 + 位置双重验证", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("数据访问", 3000, "FBE9E7"),
                        createCell("按地理位置限制敏感数据访问", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("审计追溯", 3000, "FBE9E7"),
                        createCell("操作与真实设备和位置绑定", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("GDPR 合规", 3000, "FBE9E7"),
                        createCell("最小化收集，不可逆哈希", 6026)
                    ]})
                ]
            }),

            // 7. 游戏与娱乐
            new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun("7. 游戏与娱乐")] }),

            new Table({
                width: { size: 9026, type: WidthType.DXA },
                columnWidths: [3000, 6026],
                rows: [
                    new TableRow({ children: [
                        createTableCell("场景", 3000, true),
                        createTableCell("应用", 6026, true)
                    ]}),
                    new TableRow({ children: [
                        createCell("防作弊", 3000, "FCE4EC"),
                        createCell("检测虚拟机、多开、脚本", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("跨游戏身份", 3000, "FCE4EC"),
                        createCell("统一游戏身份系统", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("成就系统", 3000, "FCE4EC"),
                        createCell("跨游戏成就累积", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("电竞反作弊", 3000, "FCE4EC"),
                        createCell("硬件指纹 + 地理位置验证", 6026)
                    ]})
                ]
            }),

            // 8. 金融服务增强
            new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun("8. 金融服务增强")] }),

            new Table({
                width: { size: 9026, type: WidthType.DXA },
                columnWidths: [3000, 6026],
                rows: [
                    new TableRow({ children: [
                        createTableCell("场景", 3000, true),
                        createTableCell("GyID 作用", 6026, true)
                    ]}),
                    new TableRow({ children: [
                        createCell("KYC 增强", 3000, "E1F5FE"),
                        createCell("设备指纹 + 地理位置辅助身份验证", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("反洗钱", 3000, "E1F5FE"),
                        createCell("地理位置与账户活动模式分析", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("信贷风控", 3000, "E1F5FE"),
                        createCell("设备稳定性和位置历史评估", 6026)
                    ]})
                ]
            }),

            // 分页
            new Paragraph({ children: [new PageBreak()] }),

            // 技术集成方式
            new Paragraph({ heading: HeadingLevel.HEADING_1, children: [new TextRun("技术集成方式")] }),

            // Web3 应用集成
            new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun("Web3 应用集成")] }),

            new Paragraph({
                shading: { fill: "F5F5F5", type: ShadingType.CLEAR },
                spacing: { before: 120, after: 120 },
                indent: { left: 360, right: 360 },
                children: [
                    new TextRun({ text: "// Web 前端集成", font: "Courier New", size: 18, color: "666666" })
                ]
            }),
            new Paragraph({
                shading: { fill: "F5F5F5", type: ShadingType.CLEAR },
                spacing: { after: 60 },
                indent: { left: 360, right: 360 },
                children: [
                    new TextRun({ text: "import init, { generate_gyid } from '@gyid/sdk-web';", font: "Courier New", size: 18, color: "0066CC" })
                ]
            }),
            new Paragraph({
                shading: { fill: "F5F5F5", type: ShadingType.CLEAR },
                spacing: { after: 60 },
                indent: { left: 360, right: 360 },
                children: [
                    new TextRun({ text: "await init();", font: "Courier New", size: 18 })
                ]
            }),
            new Paragraph({
                shading: { fill: "F5F5F5", type: ShadingType.CLEAR },
                spacing: { after: 60 },
                indent: { left: 360, right: 360 },
                children: [
                    new TextRun({ text: "const result = await generate_gyid({ geo_level: 'city' });", font: "Courier New", size: 18 })
                ]
            }),
            new Paragraph({
                shading: { fill: "F5F5F5", type: ShadingType.CLEAR },
                spacing: { after: 120 },
                indent: { left: 360, right: 360 },
                children: [
                    new TextRun({ text: "// result.id 可作为 Web3 DApp 的身份凭证", font: "Courier New", size: 18, color: "666666" })
                ]
            }),

            // 移动端集成
            new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun("移动端集成")] }),

            new Paragraph({
                shading: { fill: "F5F5F5", type: ShadingType.CLEAR },
                spacing: { before: 120, after: 60 },
                indent: { left: 360, right: 360 },
                children: [
                    new TextRun({ text: "// Android", font: "Courier New", size: 18, color: "666666" })
                ]
            }),
            new Paragraph({
                shading: { fill: "F5F5F5", type: ShadingType.CLEAR },
                spacing: { after: 120 },
                indent: { left: 360, right: 360 },
                children: [
                    new TextRun({ text: "val gyid = GyIdSdk.generateDefault()", font: "Courier New", size: 18 })
                ]
            }),

            // 企业系统集成
            new Paragraph({ heading: HeadingLevel.HEADING_2, children: [new TextRun("企业系统集成")] }),

            new Paragraph({
                shading: { fill: "F5F5F5", type: ShadingType.CLEAR },
                spacing: { before: 120, after: 60 },
                indent: { left: 360, right: 360 },
                children: [
                    new TextRun({ text: "// 后端服务", font: "Courier New", size: 18, color: "666666" })
                ]
            }),
            new Paragraph({
                shading: { fill: "F5F5F5", type: ShadingType.CLEAR },
                spacing: { after: 60 },
                indent: { left: 360, right: 360 },
                children: [
                    new TextRun({ text: "let storage = LocalStorage::new(None)?;", font: "Courier New", size: 18 })
                ]
            }),
            new Paragraph({
                shading: { fill: "F5F5F5", type: ShadingType.CLEAR },
                spacing: { after: 120 },
                indent: { left: 360, right: 360 },
                children: [
                    new TextRun({ text: "let anchors = storage.get_anchors()?;", font: "Courier New", size: 18 })
                ]
            }),

            // 市场定位
            new Paragraph({ heading: HeadingLevel.HEADING_1, children: [new TextRun("市场定位")] }),

            new Table({
                width: { size: 9026, type: WidthType.DXA },
                columnWidths: [3000, 6026],
                rows: [
                    new TableRow({ children: [
                        createTableCell("维度", 3000, true),
                        createTableCell("GyID 定位", 6026, true)
                    ]}),
                    new TableRow({ children: [
                        createCell("去中心化", 3000, "E8EAF6"),
                        createCell("优于传统 PKI，无需中心化 CA", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("隐私保护", 3000, "E8EAF6"),
                        createCell("优于 OAuth/OIDC，不收集用户数据", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("防欺诈", 3000, "E8EAF6"),
                        createCell("优于手机号/邮箱注册", 6026)
                    ]}),
                    new TableRow({ children: [
                        createCell("跨平台", 3000, "E8EAF6"),
                        createCell("优于 FIDO2/WebAuthn（需要硬件密钥）", 6026)
                    ]})
                ]
            }),

            // 推荐优先场景
            new Paragraph({ heading: HeadingLevel.HEADING_1, children: [new TextRun("推荐优先场景")] }),

            new Paragraph({
                spacing: { after: 200 },
                children: [new TextRun({ text: "基于技术成熟度和商业价值，建议按以下优先级开发：", font: "Arial", size: 22 })]
            }),

            new Table({
                width: { size: 9026, type: WidthType.DXA },
                columnWidths: [1500, 3526, 4000],
                rows: [
                    new TableRow({ children: [
                        createTableCell("优先级", 1500, true),
                        createTableCell("场景", 3526, true),
                        createTableCell("理由", 4000, true)
                    ]}),
                    new TableRow({ children: [
                        createCell("P0", 1500, "FFCDD2", true, "C62828"),
                        createCell("GEOYUAN APP 集成", 3526),
                        createCell("项目规划明确", 4000)
                    ]}),
                    new TableRow({ children: [
                        createCell("P1", 1500, "FFE0B2", true, "E65100"),
                        createCell("Web3 DApp 身份登录", 3526),
                        createCell("市场需求旺盛", 4000)
                    ]}),
                    new TableRow({ children: [
                        createCell("P2", 1500, "FFF9C4", true, "F9A825"),
                        createCell("企业安全/远程办公", 3526),
                        createCell("B2B 变现路径清晰", 4000)
                    ]}),
                    new TableRow({ children: [
                        createCell("P3", 1500, "C8E6C9", true, "2E7D32"),
                        createCell("游戏防作弊", 3526),
                        createCell("技术优势明显", 4000)
                    ]})
                ]
            }),

            // 结束语
            new Paragraph({
                alignment: AlignmentType.CENTER,
                spacing: { before: 600 },
                border: { top: { style: BorderStyle.SINGLE, size: 6, color: "2E75B6", space: 1 } },
                children: [
                    new TextRun({ text: "— 文档结束 —", font: "Arial", size: 22, color: "666666", italics: true })
                ]
            }),
        ]
    }]
});

// 生成文档
Packer.toBuffer(doc).then(buffer => {
    fs.writeFileSync("c:/Users/Nice/GYID/docs/GyID应用场景分析.docx", buffer);
    console.log("文档已生成: GyID应用场景分析.docx");
}).catch(err => {
    console.error("生成失败:", err);
});
