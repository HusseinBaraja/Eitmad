use super::super::{Client, Result, checked, correlation, deadline, mutation};
use eitmad_authorization::AuthorizationService;
use eitmad_contracts::{
    commands::CreateCustomer,
    customer::{Customer, CustomerAddress, CustomerName, CustomerNotes, CustomerPhone},
    order::{AcceptQuotation, AcceptanceMethod, ConfirmOrder, ConvertQuotation, OrderAction},
    pricing::{PriceSelection, PriceTarget},
    quotation::{EvaluateQuotation, QuotationCustomerIntent, QuotationLineIntent},
    quotation_approval::{ConfirmDiscountApproval, DiscountApprovalAction},
    quotation_draft::CreateQuotationDraft,
    quotation_lifecycle::{ConfirmQuotation, IssueQuotation, ListQuotations, QuotationAction},
    sales_catalog::CheckSalesConfiguration,
    transport::IdempotencyKey,
};
use eitmad_orders::OrderServer;
use eitmad_pricing::{DiscountApprovalServer, QuotationDraftService, QuotationServer};
use uuid::Uuid;
#[path = "fulfillment.rs"]
mod fulfillment;

pub(super) fn run(manager: &Client, reception: &Client, targets: &[PriceTarget]) -> Result<()> {
    if targets.is_empty() {
        return Err("demo published catalog empty");
    }
    let customers = customers(reception)?;
    super::super::customers::synchronize(reception)?;
    for index in 0..72_usize {
        let state = (index % 6).min(4) as u128;
        let target = &targets[index % targets.len()];
        seed_quotation(
            reception,
            target,
            &customers[index],
            index as u128,
            state,
            if state == 1 { 600 } else { 0 },
        )?;
    }
    fulfillment::run(manager, reception)
}

fn customers(reception: &Client) -> Result<Vec<Customer>> {
    let service = eitmad_customer::CustomerService::new(
        reception.store.clone(),
        AuthorizationService::new(reception.store.clone()),
    );
    let first = [
        "سالم",
        "عوض",
        "عبدالله",
        "محمد",
        "حسن",
        "أحمد",
        "عمر",
        "خالد",
        "فاطمة",
        "مريم",
        "نورة",
        "سعاد",
    ];
    let families = [
        "باحارثة",
        "باوزير",
        "بامطرف",
        "باعبيد",
        "بن محفوظ",
        "باكثير",
        "باسويدان",
        "باجابر",
    ];
    let places = [
        "المكلا ـ الديس",
        "سيئون ـ حي الوحدة",
        "تريم ـ عيديد",
        "الشحر ـ المجورة",
        "غيل باوزير ـ السوق",
        "القطن ـ حوطة أحمد",
        "شبام ـ السحيل",
        "الحامي ـ الحارة الشرقية",
        "المكلا ـ فوة",
        "روكب ـ الحي البحري",
        "الديس الشرقية ـ الشارع العام",
        "سيئون ـ القرن",
    ];
    let notes = [
        "المجلس حق الضيوف، يبغاه وسيع ومريح. اتصل قبل توصل والتحميل يكون بعد العصر.",
        "البيت في الدور الثاني والدرج ضيق؛ قس القطع قبل التجميع وخلي التركيب داخل الغرفة.",
        "غرفة زواج، الموعد مهم. الزبون طلب لون جوزي هادئ ومقابض نحاسية، وأكد على نعومة الحواف.",
        "يا خوي لا تنسى مخدات الظهر، نفس لون العينة اللي اتفقنا عليها في المعرض.",
        "الطلب للوالدة، تفضل الجلسة المرتفعة شوي. التأكيد بالتلفون قبل خروج الدينا من الورشة.",
        "معاه مقاس من البيت؛ يراجع النجار العرض والارتفاع أول، وبعدين يعتمد التفصيل على بركة الله.",
        "الشحنة لخط الوادي؛ التغليف طبقتين والزوايا محمية، والاستلام عند مدخل الحارة.",
        "مر علينا بعد المغرب واختار القماش الرملي. يريد القطع جاهزة قبل عزومة الخميس.",
    ];
    let mut customers = Vec::new();
    for (family, surname) in families.iter().enumerate() {
        for (given, name) in first.iter().enumerate() {
            let index = family * first.len() + given;
            customers.push(
                checked(
                    service.create(
                        &mutation(&reception.branch, 0x4844_4000 + index as u128),
                        &CreateCustomer {
                            name: checked(
                                CustomerName::parse(format!("{name} {surname}")),
                                "demo customer name",
                            )?,
                            phone: checked(
                                CustomerPhone::parse(format!("000-{:06}", index + 1)),
                                "demo synthetic phone",
                            )?,
                            address: Some(checked(
                                CustomerAddress::parse(format!(
                                    "{}، قرب مسجد الحي، بيت رقم {} ـ عنوان اصطناعي",
                                    places[index % places.len()],
                                    index + 11
                                )),
                                "demo address",
                            )?),
                            notes: Some(checked(
                                CustomerNotes::parse(format!(
                                    "{} بيانات اصطناعية للتجربة، الرقم غير صالح للاتصال.",
                                    notes[index % notes.len()]
                                )),
                                "demo customer notes",
                            )?),
                        },
                    ),
                    "demo customer",
                )?
                .customer,
            );
        }
    }
    Ok(customers)
}

pub(super) fn customer_count(client: &Client) -> Result<usize> {
    use eitmad_contracts::customer::{CustomerSearchTerm, SearchCustomers};
    let service = eitmad_customer::CustomerService::new(
        client.store.clone(),
        AuthorizationService::new(client.store.clone()),
    );
    let page = checked(
        service.search(
            &client.branch,
            &checked(
                SearchCustomers::new(
                    checked(CustomerSearchTerm::parse(""), "demo search")?,
                    None,
                    100,
                ),
                "demo search page",
            )?,
        ),
        "demo customer count",
    )?;
    if page.next.is_some() {
        return Err("demo customer count exceeded");
    }
    Ok(page.items.len())
}

pub(super) fn verify(manager: &Client, reception: &Client) -> Result<()> {
    use eitmad_contracts::order::ListOrders;
    for client in [manager, reception] {
        super::super::customers::synchronize(client)?;
        let quotes = checked(
            client.workflow.quotations(
                &client.branch,
                &ListQuotations {
                    after: None,
                    limit: 100,
                },
                deadline(),
            ),
            "demo quotation count",
        )?;
        let orders = checked(
            client.workflow.orders(
                &client.branch,
                &ListOrders {
                    after: None,
                    limit: 100,
                },
                None,
                deadline(),
            ),
            "demo order count",
        )?;
        if customer_count(client)? != 96 || quotes.items.len() != 72 || orders.items.len() != 24 {
            return Err("demo synchronized record counts differ");
        }
    }
    Ok(())
}

fn seed_quotation(
    reception: &Client,
    target: &PriceTarget,
    customer: &eitmad_contracts::customer::Customer,
    index: u128,
    state: u128,
    discount: u32,
) -> Result<()> {
    let auth = AuthorizationService::new(reception.store.clone());
    let drafts = QuotationDraftService::new(reception.store.clone(), auth);
    let step = 0x4844_5000 + index * 20;
    let draft = checked(
        drafts.create(
            &mutation(&reception.branch, step),
            &CreateQuotationDraft {
                intent: EvaluateQuotation {
                    customer: Some(QuotationCustomerIntent {
                        id: customer.id,
                        revision: customer.revision,
                    }),
                    lines: vec![QuotationLineIntent {
                        id: Uuid::from_u128(step),
                        configuration: configuration(reception, target, index)?,
                    }],
                    discount_basis_points: discount,
                },
            },
        ),
        "quotation draft",
    )?;
    checked(
        DiscountApprovalServer::transition(
            &reception.workflow,
            &reception.branch,
            &ConfirmDiscountApproval {
                scope: reception.branch.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::from_u128(step + 1)),
                action: if state == 1 {
                    DiscountApprovalAction::Request(draft.snapshot.clone())
                } else {
                    DiscountApprovalAction::Refresh(draft.snapshot.clone())
                },
            },
            deadline(),
        ),
        "confirm draft or request discount approval",
    )?;
    if state < 2 {
        return Ok(());
    }
    finish_quotation(reception, &drafts, &draft.snapshot, state, step)
}

fn configuration(
    client: &Client,
    target: &PriceTarget,
    index: u128,
) -> Result<CheckSalesConfiguration> {
    let details = checked(
        eitmad_pricing::PricingService::new(
            client.store.clone(),
            AuthorizationService::new(client.store.clone()),
        )
        .sales_catalog_item(
            &client.actor,
            &eitmad_contracts::sales_catalog::GetSalesCatalogItem {
                target: target.clone(),
            },
        ),
        "demo confirmed sales definition",
    )?;
    let entry = details
        .variants
        .into_iter()
        .find(|entry| entry.price.target == *target)
        .ok_or("demo sales variant")?;
    Ok(CheckSalesConfiguration {
        selection: PriceSelection {
            target: target.clone(),
            price_revision: entry.price.revision,
            quantity: 1 + u32::try_from(index % 3).map_err(|_| "demo quantity")?,
            color_id: None,
            handle_id: None,
        },
        dimensions: entry.dimensions,
    })
}

fn finish_quotation(
    reception: &Client,
    drafts: &QuotationDraftService,
    draft: &eitmad_contracts::quotation_draft::QuotationDraftSnapshot,
    index: u128,
    step: u128,
) -> Result<()> {
    let record = checked(
        reception.workflow.quotations(
            &reception.branch,
            &ListQuotations {
                after: None,
                limit: 100,
            },
            deadline(),
        ),
        "read confirmed quotation",
    )?
    .items
    .into_iter()
    .find(|r| r.quotation.id == draft.id)
    .ok_or("confirmed draft missing")?;
    let issued = checked(
        reception.workflow.quotation_transition(
            &reception.branch,
            &ConfirmQuotation {
                scope: reception.branch.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::from_u128(step + 2)),
                action: QuotationAction::Issue(IssueQuotation {
                    draft_id: record.quotation.id,
                    expected_revision: 1,
                    expected_draft_revision: draft.revision,
                }),
            },
            deadline(),
        ),
        "issue quotation",
    )?;
    checked(
        drafts.cache_quotation(&reception.branch, &issued, correlation()),
        "cache issued quotation",
    )?;
    if index == 2 {
        return Ok(());
    }
    let accepted = checked(
        reception.workflow.quotation_transition(
            &reception.branch,
            &ConfirmQuotation {
                scope: reception.branch.scope.clone(),
                idempotency_key: IdempotencyKey::new(Uuid::from_u128(step + 3)),
                action: QuotationAction::Accept(AcceptQuotation {
                    draft_id: issued.quotation.id,
                    expected_revision: issued.revision,
                    method: AcceptanceMethod::Phone,
                    note: Some("الزبون وافق على المقاس والتشطيب، قال توكلوا على الله وجهزوا الطلب بعد تأكيد موعد التركيب.".into()),
                }),
            },
            deadline(),
        ),
        "accept quotation",
    )?;
    checked(
        drafts.cache_quotation(&reception.branch, &accepted, correlation()),
        "cache accepted quotation",
    )?;
    if index == 4 {
        checked(
            OrderServer::transition(
                &reception.workflow,
                &reception.branch,
                &ConfirmOrder {
                    scope: reception.branch.scope.clone(),
                    idempotency_key: IdempotencyKey::new(Uuid::from_u128(step + 4)),
                    action: OrderAction::Convert(ConvertQuotation {
                        draft_id: accepted.quotation.id,
                        expected_revision: accepted.revision,
                    }),
                },
                deadline(),
            ),
            "convert accepted quotation to order",
        )?;
    }
    Ok(())
}
