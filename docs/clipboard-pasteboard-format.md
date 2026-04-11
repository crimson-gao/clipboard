# Finder 文件复制的真实剪切板格式

最近一次实测样本：复制 Finder 中的单个文件 `/Users/shuizhao.gh/Desktop/agno-agent-3.py`。

## 关键类型

- `public.file-url`
- `CorePasteboardFlavorType 0x6675726C`
- `NSFilenamesPboardType`
- `Apple URL pasteboard type`
- `public.utf8-plain-text`
- `NSStringPboardType`
- `com.apple.icns`
- `public.tiff`

## 关键 payload

### `public.file-url`

```text
file:///Users/shuizhao.gh/Desktop/agno-agent-3.py
```

### `NSFilenamesPboardType`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
	<string>/Users/shuizhao.gh/Desktop/agno-agent-3.py</string>
</array>
</plist>
```

### `Apple URL pasteboard type`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
	<string>file:///Users/shuizhao.gh/Desktop/agno-agent-3.py</string>
	<string></string>
</array>
</plist>
```

### `public.utf8-plain-text`

```text
agno-agent-3.py
```

## 结论

- Finder 复制文件时，剪切板里同时包含“文件引用”和“图标预览”。
- 正确判定应优先读取 `public.file-url` / `NSFilenamesPboardType` / `Apple URL pasteboard type`。
- `public.tiff` 和 `com.apple.icns` 只是图标/预览，不能据此把条目判成 `image`。
- 如果 Web 层抽象暴露的格式信息不稳定，应优先走 macOS 原生 pasteboard 读取。
