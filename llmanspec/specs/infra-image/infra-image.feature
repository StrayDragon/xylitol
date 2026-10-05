# language: zh-CN
# capability: infra-image
# purpose: 图片处理 — 格式检测、缩放、方向校正与终端渲染。
# scope: src/infra/image/

功能: infra-image

  @req:r1440
  规则: image-resize
    System MUST 提供 resize_image()，将图片缩放到可配置 max width/height/bytes 内并保持宽高比。
    # verified-by: src/infra/image/resize.rs

    场景: constrained-image-resize
      当 以限幅选项缩放内存图片
      那么 输出 base64 且宽高与字节受限
  @req:r1441
  规则: exif-orientation
    System MUST 加载图片时应用 EXIF orientation 元数据并相应校正像素数据。
    # verified-by: src/infra/image/resize.rs

    场景: exif-orientation-boundary
      当 读取图像解码方向边界
      那么 解码应用 EXIF 定向且像素校正在册
  @req:r1442
  规则: format-convert
    System MUST 支持常见图片格式（PNG、JPEG、WebP）互转，需要缩小时自动选择 JPEG 与可配置 quality。
    # verified-by: src/infra/image/resize.rs

    场景: overlimit-image-format-convert
      当 请求将超限图片转为受限格式
      那么 输出采用压缩格式编码
  @req:r1443
  规则: multi-modal-prep
    System MUST 产出适合 multimodal LLM 输入的输出：base64 编码 JPEG/PNG，载荷低于 4.5MB，含原始尺寸元数据。
    # verified-by: src/infra/image/from_path.rs

    场景: multimodal-payload-bounds
      当 从图片文件产出多模态载荷
      那么 base64 载荷在带宽上限内且含媒体类型
  @req:r1444
  规则: path-to-multimodal-image
    System MUST 提供从本地图片路径产出 multimodal ImageContent（或等价 AgentPart::Image）的入口：读取文件、经图像缩放约束尺寸与载荷、填充 media_type 与 base64 data。路径不可读或超限失败 MUST 返回描述性错误，MUST NOT 静默跳过。
    # verified-by: llmanspec/specs/agent-tools/agent-tools.feature

    场景: path-to-image-part
      当 从本地图片路径装配图片构件
      那么 得到多模态图片构件
