---
title: "استخدم مصطلحات الاعتماد المعتمدة"
description: "مرجع عربي وإنجليزي لمصطلحات الأثاث والعمل والتسعير والعقود والمزامنة والأمان."
audience: "developer"
page_type: "reference"
status: "active"
owner: "product maintainers"
last_verified: "2026-08-25"
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
| المادة الخام | Raw material | A material stocked for use before or during furniture production. The current Windows **المواد الخام** page is a preview fixture; its durable lifecycle and authority are not yet defined. |
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
| إعادة مزامنة الاشتراك | Subscription resynchronization | Shell recovery after a resume cursor cannot prove continuity. The shell opens a fresh stream and replaces query-backed view state from Rust; it does not merge or invent authority. |
| حدث | Event | A typed engine-to-shell value delivered through a subscription; it is not automatically a durable audit or event-sourcing record. |
| مؤشر استئناف الحدث | Event cursor | An opaque, scoped position used to resume a bounded event stream; it is not a globally ordered record identifier. |
| نافذة إعادة التشغيل | Replay window | The bounded same-engine event history available for subscription reconnection. |
| نافذة منع تكرار المزامنة | Sync idempotency window | The bounded durable history used to recognize retained local-change, command, and delivery identities; it is not indefinite retry storage. |
| ضغط التدفق | Backpressure | The explicit response when a consumer cannot keep pace; replaceable state may coalesce, while unreplayable discrete gaps close and require recovery. |
| نطاق | Scope | The explicit ownership and isolation boundary attached to every record and operation. |
| المستأجر | Tenant | **Provisional UI term.** The top-level identity and data-isolation root. It is not a billing tenant or a substitute for an explicit record scope. |
| مساحة العمل | Workspace | A tenant-owned collaboration or operational boundary, optionally associated with one organization. Exact user-visible use remains provisional. |
| المستخدم | User | A stable person identity independent of a specific tenant account or operational role. Name the role when behavior differs. |
| الحساب | Account | A tenant-specific binding for one user; it is not a session or authentication secret. |
| هوية الجهاز | Device identity | A stable installation-local identifier persisted by Rust. It is attribution metadata, not authentication proof. |
| جهاز مسجّل | Registered device | **Provisional UI term.** A server-known device ID bound to one public key and account. Its signed proof supports authentication; the private key never enters the server. |
| رمز المستأجر | Tenant code | **Provisional UI term.** A lowercase ASCII routing identifier entered with a username during server sign-in; it is not a tenant display name or secret. |
| الجلسة | Session | A persisted, expiring binding between principal, account, device, tenant, optional organization/workspace, and connectivity state. |
| رمز التحديث | Refresh token | **Provisional UI term.** A rotating opaque credential that may issue a new access-token pair. Reuse revokes its token family. It is not an update-channel identifier. |
| جلسة دون اتصال | Offline session | An issued, unexpired, open local session whose engine connectivity state is offline; offline state does not bypass authorization or expiry. |
| إعداد النظام | Configuration setting | A Rust-registered typed value with an authoritative default, validator, sensitivity, restart requirement, and scope. |
| لقطة الإعدادات | Configuration snapshot | The stable, revisioned, redacted projection that a shell may read; it is not a shell-owned config file. |
| مركز العمليات | Operations center | The Arabic-first Windows shell surface that presents Rust-owned health, readiness, sync, update, configuration, job, notification, and error state. It is not a control-plane authority. |
| لوحة التحكم | Dashboard | The Arabic-first Windows landing surface. When it shows **وضع المعاينة**, furniture operations values are visual fixtures and are not live Rust-owned state. |
| العلامة المرجعية | Reference marker | A product-neutral, scoped, bounded text value used to prove the complete architecture path. Rust owns validation, authorization, storage, audit, sync, and contracts. The Windows UI calls it **الميزة المرجعية**. |
| المحرك غير متاح الآن | Engine unavailable | A Windows shell availability state that means the supervised process or typed IPC channel is not usable. It does not mean authoritative data was deleted. |
| تعديل الإعدادات | Configuration patch | A typed optimistic request containing expected revision and changed keys; Rust validates and commits it atomically. |
| مراجعة الإعدادات | Configuration revision | A monotonic per-scope number incremented once by a real effective patch, not by a no-op or replay. |
| صلاحية | Permission | An allowed action derived from authenticated identity and relationships; it is enforced by Rust. |
| علاقة صلاحية | Authorization relationship | A scoped tuple connecting a principal or object to another object through one registered relation. Direct organization relationships remain the persisted policy-v1 subset. |
| ثلاثية علاقة الصلاحية | Relationship tuple | A policy edge with subject, relation, object, and optional condition. Subject and object edges cannot cross tenant/workspace boundaries. |
| كائن محدد النطاق | Scoped object | An authorization object whose tenant and optional workspace are inseparable from its kind and identifier. |
| دور مرتبط | Relationship role | A scoped role object whose members receive permissions only through explicit membership and object-relation tuples; it is not a client claim. |
| صلاحية موروثة | Inherited permission | Access granted through an explicit, bounded parent relation from a protected object to an authorized ancestor. |
| شرط الصلاحية | Authorization condition | An optional equality, `all`, or `any` predicate over canonical request attributes; a failed or absent required value denies. |
| بوابة الصلاحية والتدقيق | Authorization and audit gate | The Rust boundary that evaluates policy, durably records denials, executes only read-only callbacks through its convenience path, and leaves state-changing audit atomicity to the owning domain transaction. |
| مالك النطاق | Scope owner | A principal related as organization owner; owners administer relationships, and Rust preserves at least one persisted production owner. Explicit insecure development authentication may instead provide a non-persisted ephemeral owner. |
| مدير الإعدادات | Configuration manager | A principal related to an organization with permission to patch, import, and export configuration but not administer relationships. |
| إلغاء الصلاحية | Authorization revocation | A committed policy change that removes effective access and forces active subscriptions to reauthorize before later delivery. |
| سجل التدقيق | Audit record | A durable result envelope containing actor, device/session, tenant/workspace, scope, operation, target, outcome, time, correlation, and only redacted error metadata. |
| نقطة امتداد التدقيق | Audit extension point | A typed durable marker that classifies a common execution boundary or reserves future approval, ledger, conflict, security-event, or undo-critical linkage; a workflow marker does not implement that workflow. |
| معرّف الارتباط | Correlation ID | A non-secret identifier that connects structured operational evidence across boundaries; it is not identity, authorization, or a business record key. |
| سجل منظم | Structured log | A Rust-produced diagnostic event whose fields are allowlisted, typed, classified, correlated, and redacted before serialization. |
| التشخيص الحساس المؤقت | Temporary sensitive debug | An owner-permission-gated, localized-warning, audited mode that may reveal `Sensitive` fields until its enforced expiry, supports early disable, and always redacts `Secret` fields. No user-facing label exists yet. |
| مخزن الأسرار | Secret store | The Rust-owned capability that keeps typed secret material in an OS credential store or, only when necessary, an authenticated encrypted fallback. |
| مرجع السر | Secret reference | A typed non-secret identifier that configuration or domain state may retain instead of secret material. |
| المزامنة | Synchronization | The protocol-driven exchange and reconciliation of authorized state across processes, devices, or servers. Its business meaning does not change between simulation, LAN, direct WAN, or relay routes. |
| ناقل المزامنة | Sync transport | **Provisional UI term.** The Rust-owned connection boundary that carries one sync frame and message protocol across simulation, LAN, and WAN routes. |
| صحة اتصال المزامنة | Sync connection health | **Provisional UI term.** Operational state that reports offline, connecting, healthy, or degraded transport plus retry evidence; it does not prove reconciliation committed. |
| مرحّل المزامنة | Sync relay | **Provisional UI term.** A tenant-isolated WAN coordination route that preserves authentication and encryption when direct server connectivity is unavailable. It rejects unauthenticated or plaintext WAN sessions and cannot change sync meaning or product authorization. |
| تعارض | Conflict | Concurrent or incompatible state that cannot be safely combined without a defined domain rule or decision. |
| سجل التعارض | Conflict record | **Provisional UI term.** The durable scoped record that preserves both competing sync inputs, revisions, status, and resolution provenance. |
| محلي أولًا | Local-first | A mode in which local durable work continues offline and later synchronizes under explicit conflict rules. |
| الخادم هو المرجع | Server-authoritative | A mode in which the server determines canonical state and ordering for the domain. |
| تغيير معلّق | Pending change | A durable local-first mutation waiting for acknowledgement or reconciliation. No user-facing label is approved yet. |
| أمر معلّق | Pending command | A durable server-authoritative intent waiting for an accepted or denied server result. No user-facing label is approved yet. |
| حالة متفائلة | Optimistic state | A provisional local projection of a pending server command; it is not confirmed server truth and may roll back. No user-facing label is approved yet. |
| لقطة مزامنة | Sync snapshot | A scoped server projection with a checkpoint, generation, and explicit cache validity deadline. No user-facing label is approved yet. |
| نقطة تحقق المزامنة | Sync checkpoint | **Provisional UI term.** An opaque scoped position acknowledged after durable application and used to resume history. Its identifier bytes do not define order. |
| ذاكرة مؤقتة قديمة | Stale cache | Server-confirmed cached data whose validity deadline has passed; authoritative reads must withhold it until refresh. No user-facing label is approved yet. |
| بيانات الدمج الوصفية | Merge metadata | Provenance recording conflict strategy, common ancestor revision, source change IDs, and merge time. No user-facing label is approved yet. |
| إمكانية | Capability | A negotiated, machine-readable statement that a peer supports defined behavior or contract features. |
| معرّف البروتوكول | Protocol identifier | A stable Rust-registered operation, capability, permission, schema, error, message, or configuration name generated for native clients. |
| تفاوض الإصدار | Version negotiation | The pre-request selection of a mutually supported protocol version, capabilities, and schema versions. |
| ربط المنصة | Platform binding | Generated native-language models, serializers, and identifier constants derived from Rust contracts. |
| نافذة التوافق | Compatibility window | The protocol minor versions that released peers must continue to negotiate and pass conformance tests against. |
| انحراف العقد | Contract drift | A generated schema, fixture, reference, or platform binding that no longer exactly matches the Rust contract authority. |
| ترحيل قاعدة البيانات | Database migration | An immutable, ordered, checksummed Rust-owned schema change attributed to one storage feature. |
| انحراف المخطط | Schema drift | A difference between the live SQLite schema and the schema produced by the registered Rust migrations. |
| نسخة احتياطية | Backup | A validated SQLite copy that includes committed WAL state and has the same sensitivity as the live database. |
| استعادة قاعدة البيانات | Database restore | A stopped-engine replacement flow that validates the candidate and preserves the previous database before installation. |
| ملف الاسترداد | Recovery artifact | A preserved pre-migration, pre-restore, or failed-restore SQLite candidate that must be validated before use. |
| تصدير البيانات المحلية | Local data export | A tenant-scoped portable JSON projection that excludes sessions, devices, audit, credentials, and operational storage; it is not a backup. |
| تغيير كاسر | Breaking change | A contract representation or meaning change that a peer inside the compatibility window cannot safely interpret. |
| تحديث | Update | A signed, policy-controlled change to engine, shell, contracts, or compatible data behavior. |
| قناة التحديث | Update channel | **Provisional UI term.** The Rust-selected rollout stream. A device override takes precedence over the tenant default, which takes precedence over global `stable`. |
| مرشح الإصدار | Release candidate | **Provisional UI term.** One immutable artifact digest proposed for promotion after mandatory code, contract, security, Arabic, deployment, and recovery evidence passes. |
| ملف النشر | Deployment profile | **Provisional UI term.** A development, staging, or production set of isolated data, trust, credentials, network, update channel, and availability requirements. |
| التراجع عن الإصدار | Release rollback | **Provisional UI term.** A controlled return to compatible prior code or a complete verified pre-migration recovery point; it never means hand-editing schema history. |
| بيان تحديث موقّع | Signed update manifest | **Provisional UI term.** Immutable release, rollout, compatibility, and package metadata covered by an Ed25519 signature and verified by Rust before use. |
| طرح مرحلي | Staged rollout | **Provisional UI term.** A signed start time, pause state, deterministic device cohort, and basis-point percentage that gates update availability. |
| حالة النسخ الاحتياطي | Backup status | **Provisional UI term.** Tenant-scoped operational evidence such as current, stale, running, failed, or not configured; it is not proof of restore until restore verification succeeds. |
| سير عمل الدعم | Support workflow | **Provisional UI term.** A reason-coded, owner-authorized, audited operational action with durable running, succeeded, or failed state. |
| دورة حياة المحرك | Engine lifecycle | The Rust-owned process states `Starting`, `Ready`, `Stopping`, `Stopped`, and `Failed` plus their permitted transitions. |
| جاهزية المحرك | Engine readiness | An explicit signal that initialization completed and every required health check is healthy; it is not inferred from a live PID. |
| سلامة المحرك | Engine health | The aggregate `Healthy`, `Degraded`, or `Unhealthy` result of stable checks, evaluated separately from lifecycle state. |
| هوية عملية المحرك | Engine process identity | A launch-specific instance UUID with PID, mode, start time, product version, and protocol version; correlation metadata, not authentication. |
| سلطة المحرك | Engine authority | The single process allowed to own authoritative runtime work for one runtime directory, enforced by an operating-system lock. |
| الإشراف على عملية المحرك | Engine process supervision | Platform lifecycle coordination that launches, contains, observes, restarts, and stops an owned engine without gaining product authority. |

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
