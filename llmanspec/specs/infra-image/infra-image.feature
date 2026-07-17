# language: zh-CN
# managed by llman sdd partition-migrate
功能: infra-image

  @req:i1
  场景: scale-down
    假如 提供 4000x3000 PNG，max 2000px
    当 resize_image 被调用
    那么 输出 2000x1500 且 was_resized 为 true

  @req:i2
  场景: exif-rotate
    假如 JPEG 含 EXIF orientation 6（rotate 90）
    当 resize_image 被调用
    那么 像素数据旋转 90 度且 was_resized 为 true

  @req:i3
  场景: format-auto
    假如 PNG 大于 4.5MB，max 4.5MB
    当 resize_image 被调用
    那么 输出为 quality 80 的 JPEG 且低于 4.5MB

  @req:i4
  场景: payload
    假如 图片已处理
    当 resize_image 返回
    那么 结果含 base64 data、MIME type、原始尺寸与最终尺寸

  @req:i5
  场景: small-png-path
    假如 临时目录有小 PNG 文件
    当 path_to_image_content 被调用
    那么 返回含 data 与 image/png 的 ImageContent
