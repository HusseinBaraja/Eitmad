use super::super::{Client, Result, checked, deadline, mutation};
use eitmad_authorization::AuthorizationService;
use eitmad_contracts::{
    furniture::{
        FurnitureDimensions, FurniturePart, FurnitureState, FurnitureVariant, FurnitureVariantId,
        SaveFurniture, SaveFurnitureCategory,
    },
    material::{
        MaterialQuantity, SaveMaterial, SaveMaterialCategory, SaveMaterialUnit, UnitDimension,
    },
    part::{PartUsage, SavePart, SavePartCategory},
    pricing::{ListPrices, PriceTarget, PublishPrice},
    product::{ProductVariantId, SaveProduct, SaveProductCategory, SaveProductVariant},
};
use eitmad_pricing::{CatalogReplication as _, PricingService};
use uuid::Uuid;

pub(super) fn run(manager: &Client) -> Result<Vec<PriceTarget>> {
    let (records, unit) = materials(manager)?;
    let parts = parts(manager, &records, &unit)?;
    furniture(manager, &parts)?;
    products(manager)?;
    checked(
        manager.catalog.synchronize(&manager.actor, deadline()),
        "demo catalog upload",
    )?;
    publish(manager)
}

fn materials(
    manager: &Client,
) -> Result<(
    Vec<eitmad_contracts::material::Material>,
    eitmad_contracts::material::MaterialUnit,
)> {
    let auth = AuthorizationService::new(manager.store.clone());
    let materials = eitmad_material::MaterialService::new(manager.store.clone(), auth.clone());
    let unit = checked(
        materials.save_unit(
            &mutation(&manager.actor, 0x4844_0100),
            &SaveMaterialUnit {
                id: None,
                expected_revision: None,
                name: "وحدة توريد".into(),
                symbol: "وحدة".into(),
                dimension: UnitDimension::Count,
                numerator: 1,
                denominator: 1,
                archived: false,
            },
        ),
        "demo material unit",
    )?;
    let mut records = Vec::new();
    let names = [
        [
            "لوح سويدي ٢٤٠ سم",
            "لوح زان أحمر",
            "لوح أبلكاش ١٨ مم",
            "لوح MDF ملبس أبيض",
        ],
        [
            "قماش مخمل كحلي",
            "قماش كتان رملي",
            "قماش مجلس عنابي",
            "جلد صناعي بني",
        ],
        [
            "إسفنج ضغط عالي",
            "إسفنج ظهر مرن",
            "لباد تنجيد",
            "حشوة مخدة ناعمة",
        ],
        [
            "مفصلة هادئة",
            "سكة درج ثقيلة",
            "مقبض نحاسي",
            "رجل كنب خشبية",
        ],
        ["ورنيش شفاف", "دهان جوزي", "غراء نجارة", "معجون خشب"],
        [
            "علبة مسامير تثبيت",
            "علبة براغي تجميع",
            "رول شريط حواف",
            "كيس زوايا تقوية",
        ],
    ];
    for (group, category_name) in [
        "أخشاب وألواح",
        "أقمشة وتنجيد",
        "إسفنج وحشوات",
        "إكسسوارات وتركيبات",
        "دهانات ولواصق",
        "مستلزمات التجميع",
    ]
    .iter()
    .enumerate()
    {
        let base = 0x4844_0200 + group as u128 * 10;
        let category = checked(
            materials.save_category(
                &mutation(&manager.actor, base),
                &SaveMaterialCategory {
                    id: None,
                    expected_revision: None,
                    name: (*category_name).into(),
                    archived: false,
                },
            ),
            "demo material category",
        )?;
        for (index, name) in names[group].iter().enumerate() {
            records.push(checked(
                materials.save_material(
                    &mutation(&manager.actor, base + index as u128 + 1),
                    &SaveMaterial {
                        id: None,
                        expected_revision: None,
                        name: (*name).into(),
                        category_id: category.id,
                        unit_id: unit.id,
                        current_cost_yer: 1800
                            + i64::try_from((group * 4 + index) * 1300).map_err(|_| "demo cost")?,
                        archived: false,
                    },
                ),
                "demo material",
            )?);
        }
    }
    Ok((records, unit))
}

fn parts(
    manager: &Client,
    records: &[eitmad_contracts::material::Material],
    unit: &eitmad_contracts::material::MaterialUnit,
) -> Result<Vec<eitmad_contracts::part::Part>> {
    let service = eitmad_part::PartService::new(
        manager.store.clone(),
        AuthorizationService::new(manager.store.clone()),
    );
    let mut result = Vec::new();
    for (group, category_name) in [
        "هياكل المجالس",
        "أجزاء غرف النوم",
        "أجزاء التخزين",
        "أجزاء الطاولات",
    ]
    .iter()
    .enumerate()
    {
        let base = 0x4844_0400 + group as u128 * 10;
        let category = checked(
            service.save_category(
                &mutation(&manager.actor, base),
                &SavePartCategory {
                    id: None,
                    expected_revision: None,
                    name: (*category_name).into(),
                    archived: false,
                },
            ),
            "demo part category",
        )?;
        for (index, name) in [
            [
                "قاعدة كنب مجلس",
                "ظهر مجلس عربي",
                "ذراع كنب عريض",
                "قاعدة جلسة أرضية",
            ],
            ["رأس سرير منجد", "جانب سرير", "قاعدة مرتبة", "واجهة تسريحة"],
            ["جانب دولاب", "باب دولاب", "رف داخلي", "صندوق درج"],
            [
                "سطح طاولة وسط",
                "رجل طاولة",
                "هيكل طاولة سفرة",
                "قاعدة طاولة خدمة",
            ],
        ][group]
            .iter()
            .enumerate()
        {
            let material = &records[(group * 4 + index) % records.len()];
            result.push(
                checked(
                    service.save(
                        &mutation(&manager.actor, base + index as u128 + 1),
                        &SavePart {
                            id: None,
                            expected_revision: None,
                            name: (*name).into(),
                            category_id: category.id,
                            description:
                                "قص مضبوط، صنفرة الحواف، وفحص التربيع قبل التجميع في ورشة المكلا."
                                    .into(),
                            usages: vec![PartUsage {
                                material_id: material.id,
                                material_revision: material.revision,
                                unit_id: unit.id,
                                unit_revision: unit.revision,
                                quantity: checked(
                                    MaterialQuantity::parse("2".into()),
                                    "demo quantity",
                                )?,
                            }],
                            archived: false,
                        },
                    ),
                    "demo part",
                )?,
            );
        }
    }
    Ok(result)
}

fn furniture(manager: &Client, parts: &[eitmad_contracts::part::Part]) -> Result<()> {
    let service = eitmad_furniture::FurnitureService::new(
        manager.store.clone(),
        AuthorizationService::new(manager.store.clone()),
    );
    let names = [
        ["مجلس المكلا", "مجلس سيئون", "كنب تريم", "جلسة شبام العربية"],
        [
            "غرفة نوم الغيل",
            "سرير الشحر",
            "تسريحة الحامي",
            "كومودينو الخور",
        ],
        ["دولاب القطن", "خزانة روكب", "مكتبة فوة", "جزامة الديس"],
        [
            "سفرة حضرموت",
            "طاولة وسط السلام",
            "طاولة خدمة باعبود",
            "مكتب الطالب",
        ],
    ];
    for (group, category_name) in ["مجالس وكنب", "غرف نوم", "دواليب وتخزين", "طاولات ومكاتب"]
        .iter()
        .enumerate()
    {
        let base = 0x4844_0600 + group as u128 * 10;
        let category = checked(
            service.save_category(
                &mutation(&manager.actor, base),
                &SaveFurnitureCategory {
                    id: None,
                    expected_revision: None,
                    name: (*category_name).into(),
                    archived: false,
                },
            ),
            "demo furniture category",
        )?;
        for (index, name) in names[group].iter().enumerate() {
            let number = group * 4 + index;
            checked(service.save(&mutation(&manager.actor, base + index as u128 + 1), &SaveFurniture {
                id: None, expected_revision: None, name: (*name).into(), category_id: category.id, image: None,
                description: "تفصيل محلي للبيت الحضرمي، تشطيب جوزي هادئ وحواف ناعمة. المقاس يؤكد عند اعتماد العرض.".into(),
                notes: "يا خوي تأكد من عرض الباب ومساحة الدرج قبل التحميل. التغليف كامل والتركيب بعد العصر بالتنسيق.".into(),
                parts: vec![FurniturePart { reference: parts[number].composition.clone(), quantity: 3 }],
                variants: ["صغير", "وسط", "كبير"].iter().enumerate().map(|(variant, label)| FurnitureVariant {
                    id: FurnitureVariantId::new(Uuid::from_u128(0x4844_1000 + number as u128 * 10 + variant as u128)),
                    name: (*label).into(), dimensions: FurnitureDimensions {
                        width_mm: 1000 + u32::try_from(variant).unwrap_or(0) * 400, height_mm: [850, 1000, 2100, 760][group], depth_mm: [850, 2000, 550, 800][group],
                    }, customization: None, selling_price_yer: 280_000 + i64::try_from(number * 15_000 + variant * 50_000).unwrap_or(0),
                    archived: false, color_ids: vec![], handle_ids: vec![],
                }).collect(), colors: vec![], handles: vec![], state: FurnitureState::Active, confirm_below_cost: false,
            }), "demo furniture")?;
        }
    }
    Ok(())
}

fn products(manager: &Client) -> Result<()> {
    let service = eitmad_product::ProductService::new(
        manager.store.clone(),
        AuthorizationService::new(manager.store.clone()),
    );
    let names = [
        ["مرتبة راحة", "مرتبة الهناء", "مرتبة الضيافة"],
        ["مخدة مجلس", "مخدة ظهر", "مسند جانبي"],
        ["كرسي سفرة", "كرسي مكتب", "كرسي استراحة"],
        ["طاولة شاي جاهزة", "رف أحذية جاهز", "علاقة ملابس"],
    ];
    for (group, category_name) in [
        "مراتب جاهزة",
        "مخدات ومساند",
        "كراسي جاهزة",
        "قطع منزلية جاهزة",
    ]
    .iter()
    .enumerate()
    {
        let base = 0x4844_0800 + group as u128 * 10;
        let category = checked(
            service.save_category(
                &mutation(&manager.actor, base),
                &SaveProductCategory {
                    id: None,
                    expected_revision: None,
                    name: (*category_name).into(),
                    archived: false,
                },
            ),
            "demo product category",
        )?;
        for (index, name) in names[group].iter().enumerate() {
            let number = group * 3 + index;
            checked(service.save(&mutation(&manager.actor, base + index as u128 + 1), &SaveProduct {
                id: None, expected_revision: None, name: (*name).into(), category_id: category.id, image: None,
                description: "قطعة جاهزة من المعرض، تفحص عند التسليم ويؤكد اللون مع العميل قبل التغليف.".into(),
                notes: "جهزها مع طلب الزبون، وخلي الكرتون نظيف ومحكم لخط الوادي.".into(),
                variants: ["قياسي", "ممتاز"].iter().enumerate().map(|(variant, label)| SaveProductVariant {
                    id: ProductVariantId::new(Uuid::from_u128(0x4844_2000 + number as u128 * 10 + variant as u128)),
                    name: (*label).into(), purchase_cost_yer: 25_000 + i64::try_from(number * 6000 + variant * 10_000).unwrap_or(0), archived: false,
                }).collect(), archived: false,
            }), "demo product")?;
        }
    }
    Ok(())
}

fn publish(manager: &Client) -> Result<Vec<PriceTarget>> {
    let service = PricingService::new(
        manager.store.clone(),
        AuthorizationService::new(manager.store.clone()),
    )
    .with_confirmation(manager.prices.clone());
    let page = checked(
        service.list(
            &manager.actor,
            &ListPrices {
                term: String::new(),
                after: None,
                limit: 100,
            },
        ),
        "demo prices",
    )?;
    let mut targets = Vec::new();
    for (index, item) in page.items.into_iter().enumerate() {
        if !item.publication_required {
            targets.push(item.target);
            continue;
        }
        let amount = match &item.published {
            Some(value) => value.selling_price_yer,
            None => item.cost_yer.unwrap_or(100_000) * 14 / 10,
        };
        checked(
            service.publish(
                &mutation(&manager.actor, 0x4844_3000 + index as u128),
                &PublishPrice {
                    target: item.target.clone(),
                    expected_revision: item.published.map(|p| p.revision),
                    selling_price_yer: amount,
                    confirm_below_cost: false,
                },
                deadline(),
            ),
            "demo publish price",
        )?;
        targets.push(item.target);
    }
    Ok(targets)
}
