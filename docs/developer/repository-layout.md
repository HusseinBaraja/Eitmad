---
title: "اعثر على مالك كل جزء في المستودع"
description: "اربط كل مساحة Rust أو واجهة أو مستوى خادم أو اختبار بسلطتها وحدود امتدادها."
audience: "developer"
page_type: "reference"
status: "active"
owner: "engineering maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "a workspace member, platform, server plane, test suite, or ownership boundary changes"
keywords:
  - "ملكية المستودع"
  - "repository layout"
  - "vertical codebase"
  - "OWNERSHIP.md"
---

# اعثر على مالك كل جزء في المستودع

استخدم أقرب قدرة منتج رأسية موضعًا للسلوك الذي يتغير معًا. المساحات الحالية حدود تأسيسية فارغة تقريبًا وليست ميزات إنتاج.

## خريطة الملكية

| المساحة | السلطة الحالية | المصدر الكنسي |
| --- | --- | --- |
| `crates/contracts/` | العقود الخارجية ومدخلات التوليد أو التحقق | `crates/contracts/OWNERSHIP.md` |
| `crates/engine-runtime/` | دورة حياة المحرك وتنسيق الطلبات | `crates/engine-runtime/OWNERSHIP.md` |
| `crates/engine-cli/` | نقطة الدخول الرأسية والتشخيصية | `crates/engine-cli/OWNERSHIP.md` |
| `crates/storage/` | قاعدة البيانات والترحيلات والمعاملات والنسخ الاحتياطي | `crates/storage/OWNERSHIP.md` |
| `crates/sync/` | بروتوكول المزامنة الموحد | `crates/sync/OWNERSHIP.md` |
| `crates/authorization/` | الهوية والنطاق وتفويض ReBAC | `crates/authorization/OWNERSHIP.md` |
| `crates/update-policy/` | أهلية التحديث والتوافق وسلامة الترحيل | `crates/update-policy/OWNERSHIP.md` |
| `crates/observability-audit/` | التشخيص المنقح وتدقيق الطفرات | `crates/observability-audit/OWNERSHIP.md` |
| `crates/external-integrations/` | محولات الخدمات الخارجية المسماة | `crates/external-integrations/OWNERSHIP.md` |
| `crates/extensions/` | قدرات الامتداد وعزل المضيف | `crates/extensions/OWNERSHIP.md` |
| `shells/` | واجهات أصلية رقيقة؛ Windows يستخدم C# | `shells/OWNERSHIP.md` |
| `platform-adapters/` | ميكانيكيات نظام التشغيل القابلة للاستبدال | `platform-adapters/OWNERSHIP.md` |
| `server/*-plane/` | control وsync وrelay وupdate وadmin بحدود منفصلة | `server/OWNERSHIP.md` |
| `tests/` | اختبارات العبور بين الحدود؛ اختبارات الوحدة قرب القدرة | `tests/OWNERSHIP.md` |
| `deploy/` | أصول الحزم والنشر القابلة للتنفيذ | `deploy/OWNERSHIP.md` |
| `docs/` | شبكة معرفة حسب الجمهور والمهمة | [فهرس الوثائق](../index.md) |

## قواعد الإضافة

1. سمِّ السلوك بمصطلح منتج من [المسرد](../glossary.md).
2. ضع أنواع المجال والتحقق والتخزين والأوامر والاختبارات مع القدرة التي تملكها.
3. صدّر سطحًا صغيرًا فقط؛ استخدم الخاص أو `pub(crate)` افتراضيًا.
4. اجعل البنية التحتية العامة وراء حد صريح ولا تنقل مساعد قدرة واحدة إلى `utils` أو `common` أو `shared`.
5. وثّق السلطة والعقود والفشل والاختبارات ونقاط الامتداد في [مجموعة المطور](index.md).

## تحقق

تطابق هذه الخريطة أعضاء `Cargo.toml` وملفات `OWNERSHIP.md` الحالية في 2026-07-11. لا تدّعي الملفات المصدرية الفارغة سلوكًا لم يُنفّذ.

الخطوة التالية: [شغّل فحوص الأساس](../operations/index.md).
