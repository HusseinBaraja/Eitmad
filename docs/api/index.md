---
title: "استخدم عقود الاعتماد"
description: "اعرف حالة عقود IPC الخارجية ومصدرها وقواعد التوافق قبل إضافة عميل أو تكامل."
audience: "api"
page_type: "reference"
status: "active"
owner: "Rust contract maintainers"
last_verified: "2026-07-11"
review_triggers:
  - "a command, query, subscription, error, version, or capability is added"
  - "contract generation or validation tooling changes"
keywords:
  - "عقد"
  - "IPC contract"
  - "Command Query Subscription"
  - "capability negotiation"
---

# استخدم عقود الاعتماد

لا توجد عقود إنتاج بعد. الحزمة `crates/contracts/` هي موضع سلطة Rust المحجوز للأوامر والاستعلامات والاشتراكات والأخطاء والإصدارات والإمكانات.

## ما يمكن الاعتماد عليه الآن

- Rust هو المصدر الكنسي لكل عقد خارجي.
- يجب توليد روابط المنصات من تعريفات Rust أو التحقق منها آليًا.
- كل حد خارجي مُصدّر ويجري تفاوضًا على الإصدار والإمكانات.
- واجهات النظام الأصلية لا تنسخ DTOs أو قواعد التحقق يدويًا.

هذه قواعد معمارية مقبولة وليست عقدًا قابلًا للاستدعاء. راجع [ADR-0005](../decisions/0005-generated-or-validated-contracts.md) و[المعمارية المستهدفة](../architecture/target-architecture.md#ipc-model).

## عند إضافة أول عقد

وثّق في هذه المجموعة:

- الاسم الدقيق ونوع التفاعل: `Command` أو `Query` أو `Subscription`؛
- الإصدار والإمكانات والمدخلات والمخرجات والأخطاء والحدود؛
- الهوية وصلاحية ReBAC والنطاق والتدقيق؛
- مهلة الطلب والإلغاء وإعادة المحاولة والتوافق والفشل الجزئي؛
- مصدر التوليد أو أمر التحقق مع مثال اصطناعي صالح وآخر مرفوض.

## الملكية والتحقق

المصدر الحالي: `crates/contracts/src/lib.rs` و`crates/contracts/OWNERSHIP.md`. الملف يحتوي حاليًا على حد تأسيسي فقط، لذلك لا توجد أوامر قابلة للتشغيل أو مراجع مولدة.

الخطوة التالية: [ابدأ تطوير الاعتماد](../developer/index.md) قبل تنفيذ أول شريحة منتج رأسية.
