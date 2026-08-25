# language: zh-CN
# capability: infra-image
# purpose: 图片处理 — 格式检测、缩放、方向校正与终端渲染。
# scope: infra 层 image

功能: infra-image

  @req:i1 @human
  场景: image-resize
    - System MUST 提供 resize_image()，将图片缩放到可配置 max width/height/bytes 内并保持宽高比。

  @req:i2 @human
  场景: exif-orientation
    - System MUST 加载图片时应用 EXIF orientation 元数据并相应校正像素数据。

  @req:i3 @human
  场景: format-convert
    - System MUST 支持常见图片格式（PNG、JPEG、WebP）互转，需要缩小时自动选择 JPEG 与可配置 quality。

  @req:i4 @human
  场景: multi-modal-prep
    - System MUST 产出适合 multimodal LLM 输入的输出：base64 编码 JPEG/PNG，载荷低于 4.5MB，含原始尺寸元数据。

  @req:i5 @human
  场景: path-to-multimodal-image
    - System MUST 提供从本地图片路径产出 multimodal ImageContent（或等价 AgentPart::Image）的入口：读取文件、经图像缩放约束尺寸与载荷、填充 media_type 与 base64 data。路径不可读或超限失败 MUST 返回描述性错误，MUST NOT 静默跳过。
