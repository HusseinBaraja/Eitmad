---
title: "شغّل فحوص أساس الاعتماد"
description: "تحقق من تنسيق مساحة Rust وبنائها واختبارها وتشغيل المحرك الأولي وتدقيق الوثائق بأمان."
audience: "operations"
page_type: "task"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "workspace verification, executable behavior, deployment, backup, or recovery changes"
keywords:
  - "تشغيل فحوص الأساس"
  - "foundation verification"
  - "cargo clippy workspace"
  - "audit_docs.py"
---

# شغّل فحوص أساس الاعتماد

تتحقق هذه الخطوات من الهيكل الحالي فقط. لا توجد بعد حزمة تثبيت أو خدمة إنتاج أو قاعدة بيانات أو نسخ احتياطي أو استرداد قابل للتشغيل.

## المتطلبات

- Rust `1.85` أو أحدث متوافق مع `rust-version` في مساحة العمل؛
- Python 3 لتدقيق Markdown؛
- تشغيل الأوامر من جذر المستودع دون بيانات عميل حقيقية.

## الخطوات

1. تحقق من التنسيق:

   ```powershell
   cargo fmt --all -- --check
   ```

2. افحص جميع الأهداف:

   ```powershell
   cargo check --workspace --all-targets
   ```

3. ارفض تحذيرات Clippy:

   ```powershell
   cargo clippy --workspace --all-targets -- -D warnings
   ```

4. شغّل اختبارات مساحة العمل:

   ```powershell
   cargo test --workspace
   ```

5. شغّل نقطة الدخول التشخيصية الحالية:

   ```powershell
   cargo run -q -p eitmad-engine-cli
   ```

6. دقّق الوثائق:

   ```powershell
   python .agents/skills/maintain-project-documentation/scripts/audit_docs.py --root docs
   ```

## تحقق

يجب أن ينتهي كل أمر برمز خروج `0` دون تحذيرات. نقطة الدخول الحالية لا تطبع ناتجًا؛ هذا متوقع لأن `crates/engine-cli/src/main.rs` أساس فارغ.

## الاسترداد

إذا فشل أمر، توقف ولا تخفِ التحذير أو تتجاوز الاختبار. أصلح المصدر أو الوثيقة الكنسية ثم أعد الأمر الفاشل والمجموعة الكاملة. لا توجد حالة منتج تحتاج إلى rollback في الأساس الحالي.

## مهام مرتبطة

- [ابدأ تطوير الاعتماد](../developer/index.md)
- [راجع معيار التوثيق](../developer/contributing/documentation-standard.md)
