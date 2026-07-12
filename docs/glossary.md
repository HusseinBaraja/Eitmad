---
title: "استخدم مصطلحات الاعتماد المعتمدة"
description: "مرجع عربي وإنجليزي لمصطلحات الأثاث والعمل والتسعير والعقود والمزامنة والأمان."
audience: "developer"
page_type: "reference"
status: "active"
owner: "product maintainers"
last_verified: "2026-07-12"
review_triggers:
  - "a domain term, UI label, contract concept, or workshop synonym changes"
keywords:
  - "مسرد الاعتماد"
  - "Eitmad glossary"
  - "مصطلحات الأثاث"
  - "furniture terminology"
---

# Domain Glossary | مسرد المصطلحات

## Purpose

This glossary establishes shared language for product design, contracts, UI copy, documentation, search behavior, and future reporting. Arabic is the primary user-facing language. English names provide stable engineering vocabulary; they are not instructions to display English to users.

Terms marked **provisional** require confirmation with الاعتماد domain experts before their first production use. Domain review MUST capture local workshop language when it differs from formal Arabic.

## Business and people

| Arabic term | English term | Working definition |
| --- | --- | --- |
| الاعتماد | Eitmad / Al-Eitmad | The furniture manufacturer and the product identity in this repository. Transliteration choice for public branding remains provisional. |
| المنشأة | Organization | The legal or operational business boundary that owns people, sites, and records. |
| الفرع | Branch | A customer-facing or administrative location within the organization. |
| الورشة | Workshop | A production location where furniture is prepared or manufactured. |
| الإدارة | Management | Users responsible for oversight, policy, approvals, and operational decisions. |
| موظف الاستقبال | Receptionist | A user who receives customer requests and records initial details. |
| النجار | Carpenter | A craftsperson performing furniture manufacturing or modification work. |
| فني التركيب | Installer | A user who delivers or installs work at the customer site. |
| مشرف الورشة | Workshop supervisor | A user coordinating and supervising production work in a workshop. |
| العميل | Customer | A person or organization requesting or purchasing work. |
| المورّد | Supplier | A party supplying materials or services. |

## Sales and work lifecycle

| Arabic term | English term | Working definition |
| --- | --- | --- |
| طلب العميل | Customer request | The initial expression of a customer's need; it may precede pricing or commitment. |
| عرض السعر | Quotation | A priced proposal with scope, validity, terms, and versioned line items. |
| بند عرض السعر | Quotation line | A priced unit of product, service, material, or adjustment within a quotation. |
| الطلب | Order | A confirmed commercial request accepted for fulfillment. Avoid using this term for every incoming request. |
| أمر العمل | Work order | An authorized instruction to perform defined production, delivery, installation, or service work. |
| مهمة | Task | An assignable unit of work within a larger workflow or work order. |
| الحالة | Status | A controlled lifecycle value, not free-form progress text. |
| الأولوية | Priority | An explicit ordering signal for operational attention; it does not override authorization or safety. |
| الموافقة | Approval | An authorized decision allowing a controlled transition or exception. |
| الإلغاء | Cancellation | A controlled stop to future fulfillment while preserving history. It is not deletion. |
| التسليم | Delivery | Transfer of completed items to a customer or destination. |
| التركيب | Installation | On-site assembly or fitting of furniture. |
| إعادة العمل | Rework | Additional work required to correct or revise an item after an earlier production step. |

## Furniture and production

| Arabic term | English term | Working definition |
| --- | --- | --- |
| المنتج | Product | A sellable furniture design or standardized offering. |
| قطعة أثاث | Furniture item | A specific physical item requested, produced, delivered, or installed. |
| تصميم | Design | The specification of form, dimensions, materials, finishes, and construction details. |
| المقاسات | Dimensions | Length, width, height, depth, thickness, or other measurements with explicit units. |
| الخامة | Material | A substance or component used in production, such as wood, board, fabric, or hardware. |
| لوح خشبي | Board / panel | A sheet material used in furniture production. Exact material type remains a separate attribute. |
| الإكسسوارات | Hardware / accessories | Hinges, handles, rails, fasteners, and similar furniture components. User terminology is provisional by context. |
| التشطيب | Finish | The final surface treatment, color, coating, edge treatment, or appearance. |
| قائمة المواد | Bill of materials (BOM) | The versioned quantities of materials and components required for a defined item or design. |
| عملية إنتاج | Production operation | A defined manufacturing step performed on an item or batch. |
| مرحلة الإنتاج | Production stage | A controlled grouping or lifecycle position for production operations. |
| الهدر | Waste / scrap | Material consumed without becoming acceptable finished output; classification rules are provisional. |
| الفحص | Inspection | A recorded verification against defined criteria. |
| عيب | Defect | A failure to meet an agreed requirement or quality criterion. |

## Pricing and accounting

| Arabic term | English term | Working definition |
| --- | --- | --- |
| التسعير | Pricing | The controlled calculation or selection of a selling price. |
| التكلفة | Cost | Resource value consumed or expected to be consumed; the precise cost model is domain-specific. |
| السعر | Price | The amount offered or charged to a customer, excluding or including adjustments as explicitly defined. |
| الخصم | Discount | A controlled reduction from a price with reason and authorization where required. |
| الضريبة | Tax | A statutory amount calculated under a versioned jurisdiction rule. |
| العربون | Deposit | An amount received toward a future order balance. Legal/accounting treatment requires domain confirmation. |
| الدفعة | Payment | A recorded transfer of value against an obligation. |
| الرصيد المستحق | Outstanding balance | The calculated amount still owed at a defined point in time. |
| الفاتورة | Invoice | A ledger-grade commercial document recording amounts due under applicable rules. |
| قيد محاسبي | Journal entry | A balanced, append-safe accounting record. Corrections occur through controlled entries, not destructive edits. |

## System and architecture

| Arabic term | English term | Working definition |
| --- | --- | --- |
| محرك Rust | Rust engine | The separate process that is authoritative for domain logic, contracts, storage, authorization, sync, updates, and background work. |
| واجهة النظام الأصلية | Native shell | The platform-native desktop UI and OS adapter; a thin client of the Rust engine. |
| عقد | Contract | A typed, versioned definition at an external boundary, including requests, responses, errors, events, and capabilities. |
| أمر | Command | An authorized request for an authoritative state transition. |
| استعلام | Query | An authorized request to read a scoped projection without changing authoritative state. |
| اشتراك | Subscription | A resumable stream of state changes or progress delivered without polling. |
| نطاق | Scope | The explicit ownership and isolation boundary attached to every record and operation. |
| صلاحية | Permission | An allowed action derived from authenticated identity and relationships; it is enforced by Rust. |
| سجل التدقيق | Audit record | A durable account of a state-changing intent and outcome, including actor, scope, time, and correlation. |
| المزامنة | Synchronization | The protocol-driven exchange and reconciliation of authorized state across processes, devices, or servers. |
| تعارض | Conflict | Concurrent or incompatible state that cannot be safely combined without a defined domain rule or decision. |
| محلي أولًا | Local-first | A mode in which local durable work continues offline and later synchronizes under explicit conflict rules. |
| الخادم هو المرجع | Server-authoritative | A mode in which the server determines canonical state and ordering for the domain. |
| إمكانية | Capability | A negotiated, machine-readable statement that a peer supports defined behavior or contract features. |
| معرّف البروتوكول | Protocol identifier | A stable Rust-registered operation, capability, permission, schema, error, message, or configuration name generated for native clients. |
| تفاوض الإصدار | Version negotiation | The pre-request selection of a mutually supported protocol version, capabilities, and schema versions. |
| ربط المنصة | Platform binding | Generated native-language models, serializers, and identifier constants derived from Rust contracts. |
| نافذة التوافق | Compatibility window | The protocol minor versions that released peers must continue to negotiate and pass conformance tests against. |
| انحراف العقد | Contract drift | A generated schema, fixture, reference, or platform binding that no longer exactly matches the Rust contract authority. |
| تغيير كاسر | Breaking change | A contract representation or meaning change that a peer inside the compatibility window cannot safely interpret. |
| تحديث | Update | A signed, policy-controlled change to engine, shell, contracts, or compatible data behavior. |
| دورة حياة المحرك | Engine lifecycle | The Rust-owned process states `Starting`, `Ready`, `Stopping`, `Stopped`, and `Failed` plus their permitted transitions. |
| جاهزية المحرك | Engine readiness | An explicit signal that initialization completed and every required health check is healthy; it is not inferred from a live PID. |
| سلامة المحرك | Engine health | The aggregate `Healthy`, `Degraded`, or `Unhealthy` result of stable checks, evaluated separately from lifecycle state. |
| هوية عملية المحرك | Engine process identity | A launch-specific instance UUID with PID, mode, start time, product version, and protocol version; correlation metadata, not authentication. |
| سلطة المحرك | Engine authority | The single process allowed to own authoritative runtime work for one runtime directory, enforced by an operating-system lock. |

## Terms to avoid or qualify

| Ambiguous term | Why it is risky | Preferred approach |
| --- | --- | --- |
| مستخدم | It hides the actor's operational role. | Name the role when behavior differs; use “user” only for role-neutral behavior. |
| طلب | It can mean request, order, or work order. | Use `طلب العميل`, `الطلب`, or `أمر العمل` deliberately. |
| صلاحيات كاملة | It implies an unbounded superuser. | Name explicit relationships and capabilities. |
| حذف | It may mean cancellation, archival, tombstoning, or erasure. | State the lifecycle and retention behavior precisely. |
| تم الحفظ | It may mean staged locally, committed, audited, or synchronized. | Report the exact durable and sync state. |
| متزامن | It can hide queued or conflicted data. | Distinguish up-to-date, queued, syncing, conflicted, and failed. |
| كاش | It can become an undeclared source of truth. | State cache ownership, scope, invalidation, and bounds. |

## Glossary governance

- Product contracts and UI copy MUST use one term for one concept within a bounded domain.
- New terms MUST include Arabic, English engineering name, definition, and ambiguity notes.
- Domain experts MUST review provisional furniture, accounting, and local-workflow terminology before production use.
- Renaming a contract concept requires compatibility planning; changing only visible copy still requires Arabic UX review.
- Search synonyms MAY help discovery but MUST NOT merge distinct canonical concepts.
