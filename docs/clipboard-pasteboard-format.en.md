# Actual Pasteboard Format for Files Copied from Finder

Most recent sample: copying a single file from Finder:
`/Users/shuizhao.gh/Desktop/agno-agent-3.py`.

## Key Pasteboard Types

- `public.file-url`
- `CorePasteboardFlavorType 0x6675726C`
- `NSFilenamesPboardType`
- `Apple URL pasteboard type`
- `public.utf8-plain-text`
- `NSStringPboardType`
- `com.apple.icns`
- `public.tiff`

## Key Payloads

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

## Conclusions

- When Finder copies a file, the pasteboard includes both **file references** and **icon / preview payloads**.
- Correct detection should prioritize `public.file-url`, `NSFilenamesPboardType`, and `Apple URL pasteboard type`.
- `public.tiff` and `com.apple.icns` are only icon / preview data and should not cause the item to be classified as an `image`.
- If the web-layer abstraction exposes unstable format information, prefer reading the native macOS pasteboard directly.
