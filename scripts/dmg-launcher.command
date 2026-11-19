#!/bin/bash
clear
echo "=================================================="
echo "  مرئي (Mar2i) — إعداد التشغيل الأول وإزالة الحظر  "
echo "=================================================="
echo ""
echo "جارٍ إزالة علامة الحظر (Gatekeeper Quarantine)..."
xattr -cr /Applications/Mar2i.app 2>/dev/null || true
DIR="$(cd "$(dirname "$0")" && pwd)"
xattr -cr "$DIR/Mar2i.app" 2>/dev/null || true

echo "✓ تم فك الحظر بنجاح!"
echo "جارٍ فتح التطبيق الآن..."
open /Applications/Mar2i.app 2>/dev/null || open "$DIR/Mar2i.app" 2>/dev/null || true
