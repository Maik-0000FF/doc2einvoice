// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! Invoices for the tests: complete, with every business term the syntax can
//! write given a value, and with amounts that add up as the rules of EN 16931
//! demand.
//!
//! One line: 2 units at a net price of 50.00 (gross 55.00, discount 5.00),
//! an allowance of 10 % on 100.00 and a charge of 5 % on 100.00, so a net
//! amount of 95.00. On document level an allowance of 10 % on 50.00 and a
//! charge of 10 % on 100.00: 95.00 - 5.00 + 10.00 = 100.00 without VAT,
//! 19 % VAT of 19.00, 119.00 with VAT, 20.00 paid, 99.00 due.

#![allow(dead_code)]

use std::str::FromStr;

use doc2einvoice_model::{
    Amount, BinaryObject, Buyer, Code, Contact, CountryCode, CreditTransfer, CurrencyCode, Date,
    DeliveryInformation, DirectDebit, DocumentAdjustment, DocumentTotals, Identifier, Invoice,
    InvoiceLine, InvoiceNote, ItemAttribute, ItemClassification, ItemInformation, LineAdjustment,
    LineVat, NonEmpty, OptionalSchemeIdentifier, Payee, PaymentInstructions, Percentage, Period,
    PostalAddress, PrecedingInvoice, PriceDetails, ProcessControl, Quantity,
    RequiredSchemeIdentifier, Seller, SupportingDocument, TaxRepresentative, UnitPriceAmount,
    VatBreakdown, VatCategory, VatPoint,
};
use doc2einvoice_validation::{Profile, ValidationReport, Validator};
use rust_decimal::Decimal;

/// The specification identifier of EN 16931 (BT-24).
pub const EN16931: &str = "urn:cen.eu:en16931:2017";
/// The specification identifier of the CIUS XRechnung 3.0 (BT-24).
pub const XRECHNUNG: &str = "urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0";
/// The default business process of the Peppol network (BT-23).
const BUSINESS_PROCESS: &str = "urn:fdc:peppol.eu:2017:poacc:billing:01:1.0";

/// A credit note (UNTDID 1001).
pub const CREDIT_NOTE: &str = "381";

/// Codes of UNTDID 4461: SEPA credit transfer and SEPA direct debit.
const SEPA_CREDIT_TRANSFER: &str = "58";
const SEPA_DIRECT_DEBIT: &str = "59";
/// The scheme of a Global Location Number (ISO 6523).
const GLN: &str = "0088";
/// The scheme of a GTIN (ISO 6523).
const GTIN: &str = "0160";
/// The electronic address scheme of an email address (EAS).
const EMAIL: &str = "EM";
/// The unit "one" (UN/ECE Recommendation 20).
const UNIT: &str = "C62";
/// The document code qualifier for an order reference (UNTDID 1153).
const OBJECT_SCHEME: &str = "AAA";
/// The item classification scheme (UNTDID 7143).
const CLASSIFICATION_SCHEME: &str = "STI";

pub fn decimal(text: &str) -> Decimal {
    Decimal::from_str(text).unwrap_or_else(|error| unreachable!("{text}: {error}"))
}

pub fn amount(text: &str) -> Amount {
    Amount::new(decimal(text)).unwrap_or_else(|error| unreachable!("{text}: {error}"))
}

fn percent(text: &str) -> Percentage {
    Percentage(decimal(text))
}

pub fn date(year: i32, month: u32, day: u32) -> Date {
    Date::from_ymd_opt(year, month, day).unwrap_or_else(|| unreachable!("{year}-{month}-{day}"))
}

fn scheme(content: &str, scheme: &str) -> OptionalSchemeIdentifier {
    OptionalSchemeIdentifier {
        content: content.to_owned(),
        scheme: Some(scheme.to_owned()),
    }
}

fn country(code: &str) -> CountryCode {
    CountryCode::new(code).unwrap_or_else(|error| unreachable!("{error}"))
}

fn currency(code: &str) -> CurrencyCode {
    CurrencyCode::new(code).unwrap_or_else(|error| unreachable!("{error}"))
}

fn address(street: &str, city: &str) -> PostalAddress {
    PostalAddress {
        line_1: Some(street.to_owned()),
        line_2: Some("Hinterhaus".to_owned()),
        line_3: Some("3. Stock".to_owned()),
        city: Some(city.to_owned()),
        post_code: Some("10115".to_owned()),
        country_subdivision: Some("Berlin".to_owned()),
        country: country("DE"),
    }
}

fn contact(name: &str) -> Contact {
    Contact {
        point: Some(name.to_owned()),
        telephone: Some("+49 30 1234567".to_owned()),
        email: Some("kontakt@example.com".to_owned()),
    }
}

/// How the complete invoice is to be paid.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Payment {
    /// A SEPA credit transfer.
    CreditTransfer,
    /// A SEPA direct debit.
    DirectDebit,
}

fn payment(kind: Payment) -> PaymentInstructions {
    match kind {
        Payment::CreditTransfer => PaymentInstructions {
            means_code: Code(SEPA_CREDIT_TRANSFER.to_owned()),
            means_text: Some("Überweisung".to_owned()),
            remittance_information: Some("RE-2026-0042".to_owned()),
            credit_transfers: vec![CreditTransfer {
                account: Identifier("DE75512108001245126199".to_owned()),
                account_name: Some("Muster GmbH".to_owned()),
                service_provider: Some(Identifier("SOGEDEFFXXX".to_owned())),
            }],
            card: None,
            direct_debit: None,
        },
        Payment::DirectDebit => PaymentInstructions {
            means_code: Code(SEPA_DIRECT_DEBIT.to_owned()),
            means_text: Some("Lastschrift".to_owned()),
            remittance_information: Some("RE-2026-0042".to_owned()),
            credit_transfers: Vec::new(),
            card: None,
            direct_debit: Some(DirectDebit {
                mandate_reference: Some(Identifier("MANDAT-7".to_owned())),
                creditor_identifier: Some(Identifier("DE98ZZZ09999999999".to_owned())),
                debited_account: Some(Identifier("DE02120300000000202051".to_owned())),
            }),
        },
    }
}

fn line() -> InvoiceLine {
    InvoiceLine {
        identifier: Identifier("1".to_owned()),
        note: Some("Positionsnotiz".to_owned()),
        object_identifier: Some(scheme("ZÄHLER-4711", OBJECT_SCHEME)),
        quantity: Quantity(decimal("2")),
        unit: Code(UNIT.to_owned()),
        net_amount: amount("95"),
        purchase_order_line: Some("10".to_owned()),
        buyer_accounting_reference: Some("KST-4711".to_owned()),
        period: Some(Period {
            start: Some(date(2026, 10, 1)),
            end: Some(date(2026, 10, 15)),
        }),
        allowances: vec![LineAdjustment {
            amount: amount("10"),
            base_amount: Some(amount("100")),
            percentage: Some(percent("10")),
            reason: Some("Rabatt".to_owned()),
            reason_code: Some(Code("95".to_owned())),
        }],
        charges: vec![LineAdjustment {
            amount: amount("5"),
            base_amount: Some(amount("100")),
            percentage: Some(percent("5")),
            reason: Some("Fracht".to_owned()),
            reason_code: Some(Code("FC".to_owned())),
        }],
        price: PriceDetails {
            net_price: UnitPriceAmount(decimal("50")),
            discount: Some(UnitPriceAmount(decimal("5"))),
            gross_price: Some(UnitPriceAmount(decimal("55"))),
            base_quantity: Some(Quantity(decimal("1"))),
            base_quantity_unit: Some(Code(UNIT.to_owned())),
        },
        vat: LineVat {
            category: VatCategory::StandardRate,
            rate: Some(percent("19")),
        },
        item: ItemInformation {
            name: "Prüfgerät".to_owned(),
            description: Some("Ein Gerät zum Prüfen".to_owned()),
            seller_identifier: Some(Identifier("ART-1".to_owned())),
            buyer_identifier: Some(Identifier("KUNDE-ART-1".to_owned())),
            standard_identifier: Some(RequiredSchemeIdentifier {
                content: "4012345678901".to_owned(),
                scheme: GTIN.to_owned(),
            }),
            classifications: vec![ItemClassification {
                content: "12345".to_owned(),
                scheme: CLASSIFICATION_SCHEME.to_owned(),
                scheme_version: Some("1.0".to_owned()),
            }],
            country_of_origin: Some(country("DE")),
            attributes: vec![ItemAttribute {
                name: "Farbe".to_owned(),
                value: "rot".to_owned(),
            }],
        },
    }
}

/// An invoice in which every business term has a value, paid in the given
/// way, following the given specification (BT-24).
/// The seller of the complete invoice.
fn complete_seller() -> Seller {
    Seller {
        name: "Muster GmbH".to_owned(),
        trading_name: Some("Muster".to_owned()),
        identifiers: vec![scheme("4000001000005", GLN)],
        legal_registration: Some(scheme("HRB 12345", GLN)),
        vat_identifier: Some(Identifier("DE123456789".to_owned())),
        tax_registration: Some(Identifier("201/113/40209".to_owned())),
        additional_legal_information: Some("Geschäftsführung: Erika Muster".to_owned()),
        electronic_address: Some(RequiredSchemeIdentifier {
            content: "rechnung@example.com".to_owned(),
            scheme: EMAIL.to_owned(),
        }),
        address: address("Musterstraße 1", "Berlin"),
        contact: Some(contact("Buchhaltung")),
    }
}

/// The buyer of the complete invoice.
fn complete_buyer() -> Buyer {
    Buyer {
        name: "Beispiel AG".to_owned(),
        trading_name: Some("Beispiel".to_owned()),
        identifier: Some(scheme("4000001000012", GLN)),
        legal_registration: Some(scheme("HRB 54321", GLN)),
        vat_identifier: Some(Identifier("DE987654321".to_owned())),
        electronic_address: Some(RequiredSchemeIdentifier {
            content: "eingang@example.com".to_owned(),
            scheme: EMAIL.to_owned(),
        }),
        address: address("Beispielweg 2", "Berlin"),
        contact: Some(contact("Einkauf")),
    }
}

/// The totals of the complete invoice.
fn complete_totals() -> DocumentTotals {
    DocumentTotals {
        line_net_total: amount("95"),
        allowance_total: Some(amount("5")),
        charge_total: Some(amount("10")),
        total_without_vat: amount("100"),
        vat_total: Some(amount("19")),
        vat_total_in_accounting_currency: Some(amount("20.5")),
        total_with_vat: amount("119"),
        paid: Some(amount("20")),
        rounding: Some(amount("0")),
        due_for_payment: amount("99"),
    }
}

pub fn complete(kind: Payment, specification: &str) -> Invoice {
    Invoice {
        number: Identifier("RE-2026-0042".to_owned()),
        issue_date: date(2026, 11, 2),
        type_code: Code("380".to_owned()),
        currency: currency("EUR"),
        vat_accounting_currency: Some(currency("USD")),
        vat_point: Some(VatPoint::Date(date(2026, 10, 31))),
        payment_due_date: Some(date(2026, 11, 30)),
        buyer_reference: Some("04011000-12345-34".to_owned()),
        project_reference: Some("PROJ-1".to_owned()),
        contract_reference: Some("VERTRAG-1".to_owned()),
        purchase_order_reference: Some("BESTELLUNG-1".to_owned()),
        sales_order_reference: Some("AUFTRAG-1".to_owned()),
        receiving_advice_reference: Some("WE-1".to_owned()),
        despatch_advice_reference: Some("LS-1".to_owned()),
        tender_or_lot_reference: Some("LOS-1".to_owned()),
        invoiced_object: Some(scheme("ABO-99", OBJECT_SCHEME)),
        buyer_accounting_reference: Some("4711-0815".to_owned()),
        payment_terms: Some("Zahlbar innerhalb von 28 Tagen.".to_owned()),
        notes: vec![InvoiceNote {
            subject_code: Some(Code("AAI".to_owned())),
            text: "Eine Notiz zur Rechnung".to_owned(),
        }],
        process_control: ProcessControl {
            business_process: Some(BUSINESS_PROCESS.to_owned()),
            specification: Identifier(specification.to_owned()),
        },
        preceding_invoices: vec![PrecedingInvoice {
            reference: "RE-2026-0041".to_owned(),
            issue_date: Some(date(2026, 10, 2)),
        }],
        seller: complete_seller(),
        buyer: complete_buyer(),
        payee: Some(Payee {
            name: "Muster Factoring GmbH".to_owned(),
            identifier: Some(scheme("4000001000029", GLN)),
            legal_registration: Some(scheme("HRB 99999", GLN)),
        }),
        tax_representative: Some(TaxRepresentative {
            name: "Steuer Vertreter GmbH".to_owned(),
            vat_identifier: Identifier("DE111111111".to_owned()),
            address: address("Vertreterplatz 3", "Berlin"),
        }),
        delivery: Some(DeliveryInformation {
            party_name: Some("Lager Nord".to_owned()),
            location: Some(scheme("4000001000036", GLN)),
            actual_date: Some(date(2026, 10, 15)),
            invoicing_period: Some(Period {
                start: Some(date(2026, 10, 1)),
                end: Some(date(2026, 10, 31)),
            }),
            address: Some(address("Lagerstraße 4", "Berlin")),
        }),
        payment_instructions: Some(payment(kind)),
        allowances: vec![DocumentAdjustment {
            amount: amount("5"),
            base_amount: Some(amount("50")),
            percentage: Some(percent("10")),
            vat_category: VatCategory::StandardRate,
            vat_rate: Some(percent("19")),
            reason: Some("Rabatt".to_owned()),
            reason_code: Some(Code("95".to_owned())),
        }],
        charges: vec![DocumentAdjustment {
            amount: amount("10"),
            base_amount: Some(amount("100")),
            percentage: Some(percent("10")),
            vat_category: VatCategory::StandardRate,
            vat_rate: Some(percent("19")),
            reason: Some("Fracht".to_owned()),
            reason_code: Some(Code("FC".to_owned())),
        }],
        totals: complete_totals(),
        vat_breakdown: NonEmpty::one(VatBreakdown {
            taxable_amount: amount("100"),
            tax_amount: amount("19"),
            category: VatCategory::StandardRate,
            rate: Some(percent("19")),
            exemption_reason: None,
            exemption_reason_code: None,
        }),
        supporting_documents: vec![SupportingDocument {
            reference: "ANLAGE-1".to_owned(),
            description: Some("Leistungsnachweis".to_owned()),
            external_location: Some("https://example.com/anlage-1.pdf".to_owned()),
            attachment: Some(BinaryObject {
                content: b"%PDF-1.4\n%EOF\n".to_vec(),
                mime_code: "application/pdf".to_owned(),
                filename: "anlage-1.pdf".to_owned(),
            }),
        }],
        lines: NonEmpty::one(line()),
    }
}

/// Validates `document` and returns every finding of the XML Schema and of
/// the rules, of any severity, as one text per finding.
pub fn findings(document: &[u8], profile: Profile) -> Vec<String> {
    let text = std::str::from_utf8(document).unwrap_or_else(|error| unreachable!("{error}"));
    let report: ValidationReport = Validator::new()
        .validate(text, profile)
        .unwrap_or_else(|error| unreachable!("not validated: {error}\n{text}"));
    let mut all: Vec<String> = Vec::new();
    if !report.is_schema_valid() {
        all.push(format!("{report:?}"));
    }
    all.extend(report.findings().map(|finding| {
        format!(
            "{} {:?} at {}: {}",
            finding.rule_id, finding.severity, finding.location, finding.message
        )
    }));
    all
}

/// A payment by card (UNTDID 4461).
const CARD_PAYMENT: &str = "48";

/// An invoice paid by card, which UBL cannot write and CII can.
pub fn paid_by_card(specification: &str) -> Invoice {
    let mut invoice = complete(Payment::CreditTransfer, specification);
    invoice.payment_instructions = Some(PaymentInstructions {
        means_code: Code(CARD_PAYMENT.to_owned()),
        means_text: Some("Kartenzahlung".to_owned()),
        remittance_information: None,
        credit_transfers: Vec::new(),
        card: Some(doc2einvoice_model::PaymentCard {
            primary_account_number: "1234".to_owned(),
            holder_name: Some("Erika Muster".to_owned()),
        }),
        direct_debit: None,
    });
    invoice
}

/// The one line of the minimal invoice: one service at 100.00.
fn minimal_line() -> InvoiceLine {
    let mut line = line();
    line.note = None;
    line.object_identifier = None;
    line.quantity = Quantity(decimal("1"));
    line.net_amount = amount("100");
    line.purchase_order_line = None;
    line.buyer_accounting_reference = None;
    line.period = None;
    line.allowances = Vec::new();
    line.charges = Vec::new();
    line.price = PriceDetails {
        net_price: UnitPriceAmount(decimal("100")),
        discount: None,
        gross_price: None,
        base_quantity: None,
        base_quantity_unit: None,
    };
    line.item = ItemInformation {
        name: "Leistung".to_owned(),
        description: None,
        seller_identifier: None,
        buyer_identifier: None,
        standard_identifier: None,
        classifications: Vec::new(),
        country_of_origin: None,
        attributes: Vec::new(),
    };
    line
}

/// An invoice with only what EN 16931 requires, and the VAT total, which UBL
/// requires: one line of 100.00 at 19 %.
/// The totals of the minimal invoice.
fn minimal_totals() -> DocumentTotals {
    DocumentTotals {
        line_net_total: amount("100"),
        allowance_total: None,
        charge_total: None,
        total_without_vat: amount("100"),
        vat_total: Some(amount("19")),
        vat_total_in_accounting_currency: None,
        total_with_vat: amount("119"),
        paid: None,
        rounding: None,
        due_for_payment: amount("119"),
    }
}

pub fn minimal() -> Invoice {
    let bare_address = PostalAddress {
        line_1: None,
        line_2: None,
        line_3: None,
        city: None,
        post_code: None,
        country_subdivision: None,
        country: country("DE"),
    };
    let line = minimal_line();
    Invoice {
        number: Identifier("RE-1".to_owned()),
        issue_date: date(2026, 11, 2),
        type_code: Code("380".to_owned()),
        currency: currency("EUR"),
        vat_accounting_currency: None,
        vat_point: None,
        payment_due_date: None,
        buyer_reference: None,
        project_reference: None,
        contract_reference: None,
        purchase_order_reference: None,
        sales_order_reference: None,
        receiving_advice_reference: None,
        despatch_advice_reference: None,
        tender_or_lot_reference: None,
        invoiced_object: None,
        buyer_accounting_reference: None,
        payment_terms: Some("Zahlbar sofort.".to_owned()),
        notes: Vec::new(),
        process_control: ProcessControl {
            business_process: None,
            specification: Identifier(EN16931.to_owned()),
        },
        preceding_invoices: Vec::new(),
        seller: Seller {
            name: "Muster GmbH".to_owned(),
            trading_name: None,
            identifiers: Vec::new(),
            legal_registration: None,
            vat_identifier: Some(Identifier("DE123456789".to_owned())),
            tax_registration: None,
            additional_legal_information: None,
            electronic_address: None,
            address: bare_address.clone(),
            contact: None,
        },
        buyer: Buyer {
            name: "Beispiel AG".to_owned(),
            trading_name: None,
            identifier: None,
            legal_registration: None,
            vat_identifier: None,
            electronic_address: None,
            address: bare_address,
            contact: None,
        },
        payee: None,
        tax_representative: None,
        delivery: None,
        payment_instructions: None,
        allowances: Vec::new(),
        charges: Vec::new(),
        totals: minimal_totals(),
        vat_breakdown: NonEmpty::one(VatBreakdown {
            taxable_amount: amount("100"),
            tax_amount: amount("19"),
            category: VatCategory::StandardRate,
            rate: Some(percent("19")),
            exemption_reason: None,
            exemption_reason_code: None,
        }),
        supporting_documents: Vec::new(),
        lines: NonEmpty::one(line),
    }
}

/// A code of UNTDID 2005 for the VAT point date (BT-8): the invoice date.
const VAT_POINT_CODE: &str = "3";

/// The complete invoice with the VAT point given as a code (BT-8) instead of a
/// date (BT-7); the two exclude each other.
pub fn with_vat_point_code(specification: &str) -> Invoice {
    let mut invoice = complete(Payment::CreditTransfer, specification);
    invoice.vat_point = Some(VatPoint::Code(Code(VAT_POINT_CODE.to_owned())));
    invoice
}

/// The exemption from VAT for activities in the public interest (VATEX).
const EXEMPTION_CODE: &str = "VATEX-EU-132";

/// The minimal invoice, exempt from VAT with a reason in words and as a code
/// (BT-120, BT-121): 100.00 without VAT, 0.00 VAT.
pub fn exempt() -> Invoice {
    let mut invoice = minimal();
    let mut line = invoice.lines.first().clone();
    line.vat = LineVat {
        category: VatCategory::Exempt,
        rate: Some(percent("0")),
    };
    invoice.lines = NonEmpty::one(line);
    invoice.vat_breakdown = NonEmpty::one(VatBreakdown {
        taxable_amount: amount("100"),
        tax_amount: amount("0"),
        category: VatCategory::Exempt,
        rate: Some(percent("0")),
        exemption_reason: Some("Steuerfrei nach Artikel 132 der Richtlinie 2006/112/EG".to_owned()),
        exemption_reason_code: Some(Code(EXEMPTION_CODE.to_owned())),
    });
    invoice.totals.vat_total = Some(amount("0"));
    invoice.totals.total_with_vat = amount("100");
    invoice.totals.due_for_payment = amount("100");
    invoice
}

/// The value of a business term that names the term, so that a test finds
/// it again where the term is written and nowhere else.
pub mod named {
    use doc2einvoice_model::{Amount, Date, Percentage, Quantity, UnitPriceAmount};
    use rust_decimal::Decimal;

    use super::{amount as amount_of, date, decimal};

    /// The year the date of the business term 0 would lie in: the date of a
    /// term is the first of January, as many years later as its number.
    const FIRST_YEAR: i32 = 2000;
    /// The decimals of every number that names a term: the number of the
    /// term goes before them.
    const DECIMALS: &str = ".5";

    /// The text of the term `number`: its identifier, as `BT-27`.
    pub fn text(number: u16) -> String {
        format!("BT-{number}")
    }

    /// The number of the term `number`, as `27.5`.
    pub fn number(number: u16) -> Decimal {
        decimal(&format!("{number}{DECIMALS}"))
    }

    pub fn amount(term: u16) -> Amount {
        amount_of(&format!("{term}{DECIMALS}"))
    }

    pub fn percentage(term: u16) -> Percentage {
        Percentage(number(term))
    }

    pub fn quantity(term: u16) -> Quantity {
        Quantity(number(term))
    }

    pub fn price(term: u16) -> UnitPriceAmount {
        UnitPriceAmount(number(term))
    }

    /// The date of the term `number`, as the first of January 2027.
    pub fn day(number: u16) -> Date {
        date(FIRST_YEAR + i32::from(number), 1, 1)
    }

    /// Whether `written` names the term `term`: as its text, its number or
    /// its date, the date with or without the hyphens of ISO 8601.
    ///
    /// | Written      | Term | Names it |
    /// |--------------|------|----------|
    /// | `BT-27`      | 27   | yes      |
    /// | `27.5`       | 27   | yes      |
    /// | `27.50`      | 27   | yes      |
    /// | `2027-01-01` | 27   | yes      |
    /// | `20270101`   | 27   | yes      |
    /// | `BT-28`      | 27   | no       |
    /// | `BT-270`     | 27   | no       |
    /// | `28.5`       | 27   | no       |
    /// | `27`         | 27   | no       |
    /// | `2028-01-01` | 27   | no       |
    /// | nothing      | 27   | no       |
    pub fn names(written: &str, term: u16) -> bool {
        let as_number = written
            .parse::<Decimal>()
            .is_ok_and(|value| value == number(term));
        let as_date = written.replace('-', "") == day(term).format("%Y%m%d").to_string();
        written == text(term) || as_number || as_date
    }
}

/// The countries of the parties of the invoice that names its terms, each
/// its own: of the seller (BT-40), the buyer (BT-55), the tax representative
/// (BT-69), the delivery (BT-80) and the origin of the item (BT-159).
pub const NAMED_COUNTRIES: [(u16, &str); 5] =
    [(40, "DE"), (55, "FR"), (69, "IT"), (80, "NL"), (159, "ES")];

/// The currencies of the invoice that names its terms: of the invoice (BT-5)
/// and of the VAT accounting (BT-6).
pub const NAMED_CURRENCIES: [(u16, &str); 2] = [(5, "EUR"), (6, "USD")];

/// The VAT categories of the invoice that names its terms, each its own: of
/// the allowance (BT-95), the charge (BT-102), the breakdown (BT-118) and
/// the line (BT-151), with the codes they are written as.
pub const NAMED_CATEGORIES: [(u16, VatCategory, &str); 4] = [
    (95, VatCategory::ZeroRated, "Z"),
    (102, VatCategory::Exempt, "E"),
    (118, VatCategory::StandardRate, "S"),
    (151, VatCategory::ReverseCharge, "AE"),
];

/// The kind of the invoice that names its terms (BT-3): a commercial
/// invoice.
pub const NAMED_TYPE: &str = "380";

fn named_country(term: u16) -> CountryCode {
    let code = NAMED_COUNTRIES
        .iter()
        .find(|(number, _)| *number == term)
        .map_or_else(
            || unreachable!("no country of BT-{term}"),
            |(_, code)| *code,
        );
    country(code)
}

fn named_currency(term: u16) -> CurrencyCode {
    let code = NAMED_CURRENCIES
        .iter()
        .find(|(number, _)| *number == term)
        .map_or_else(
            || unreachable!("no currency of BT-{term}"),
            |(_, code)| *code,
        );
    currency(code)
}

fn named_category(term: u16) -> VatCategory {
    NAMED_CATEGORIES
        .iter()
        .find(|(number, _, _)| *number == term)
        .map_or_else(
            || unreachable!("no category of BT-{term}"),
            |(_, category, _)| *category,
        )
}

/// An identifier that names the term, in a scheme of the same name.
fn named_scheme(term: u16) -> OptionalSchemeIdentifier {
    scheme(&named::text(term), &named::text(term))
}

fn named_required_scheme(term: u16) -> RequiredSchemeIdentifier {
    RequiredSchemeIdentifier {
        content: named::text(term),
        scheme: named::text(term),
    }
}

/// An address whose parts name the terms `[line 1, line 2, line 3, city,
/// post code, subdivision, country]`.
fn named_address(terms: [u16; 7]) -> PostalAddress {
    let [
        line_1,
        line_2,
        line_3,
        city,
        post_code,
        subdivision,
        country,
    ] = terms;
    PostalAddress {
        line_1: Some(named::text(line_1)),
        line_2: Some(named::text(line_2)),
        line_3: Some(named::text(line_3)),
        city: Some(named::text(city)),
        post_code: Some(named::text(post_code)),
        country_subdivision: Some(named::text(subdivision)),
        country: named_country(country),
    }
}

fn named_contact(terms: [u16; 3]) -> Contact {
    let [point, telephone, email] = terms;
    Contact {
        point: Some(named::text(point)),
        telephone: Some(named::text(telephone)),
        email: Some(named::text(email)),
    }
}

/// An allowance or a charge of a line whose parts name the terms `[amount,
/// base amount, percentage, reason, reason code]`.
fn named_line_adjustment(terms: [u16; 5]) -> LineAdjustment {
    let [amount, base_amount, percentage, reason, reason_code] = terms;
    LineAdjustment {
        amount: named::amount(amount),
        base_amount: Some(named::amount(base_amount)),
        percentage: Some(named::percentage(percentage)),
        reason: Some(named::text(reason)),
        reason_code: Some(Code(named::text(reason_code))),
    }
}

/// An allowance or a charge of the document whose parts name the terms
/// `[amount, base amount, percentage, category, rate, reason, reason code]`.
fn named_adjustment(terms: [u16; 7]) -> DocumentAdjustment {
    let [
        amount,
        base_amount,
        percentage,
        category,
        rate,
        reason,
        reason_code,
    ] = terms;
    DocumentAdjustment {
        amount: named::amount(amount),
        base_amount: Some(named::amount(base_amount)),
        percentage: Some(named::percentage(percentage)),
        vat_category: named_category(category),
        vat_rate: Some(named::percentage(rate)),
        reason: Some(named::text(reason)),
        reason_code: Some(Code(named::text(reason_code))),
    }
}

fn named_line() -> InvoiceLine {
    InvoiceLine {
        identifier: Identifier(named::text(126)),
        note: Some(named::text(127)),
        object_identifier: Some(named_scheme(128)),
        quantity: named::quantity(129),
        unit: Code(named::text(130)),
        net_amount: named::amount(131),
        purchase_order_line: Some(named::text(132)),
        buyer_accounting_reference: Some(named::text(133)),
        period: Some(Period {
            start: Some(named::day(134)),
            end: Some(named::day(135)),
        }),
        allowances: vec![named_line_adjustment([136, 137, 138, 139, 140])],
        charges: vec![named_line_adjustment([141, 142, 143, 144, 145])],
        price: PriceDetails {
            net_price: named::price(146),
            discount: Some(named::price(147)),
            gross_price: Some(named::price(148)),
            base_quantity: Some(named::quantity(149)),
            base_quantity_unit: Some(Code(named::text(150))),
        },
        vat: LineVat {
            category: named_category(151),
            rate: Some(named::percentage(152)),
        },
        item: ItemInformation {
            name: named::text(153),
            description: Some(named::text(154)),
            seller_identifier: Some(Identifier(named::text(155))),
            buyer_identifier: Some(Identifier(named::text(156))),
            standard_identifier: Some(named_required_scheme(157)),
            classifications: vec![ItemClassification {
                content: named::text(158),
                scheme: named::text(158),
                scheme_version: Some(named::text(158)),
            }],
            country_of_origin: Some(named_country(159)),
            attributes: vec![ItemAttribute {
                name: named::text(160),
                value: named::text(161),
            }],
        },
    }
}

fn named_seller() -> Seller {
    Seller {
        name: named::text(27),
        trading_name: Some(named::text(28)),
        identifiers: vec![named_scheme(29)],
        legal_registration: Some(named_scheme(30)),
        vat_identifier: Some(Identifier(named::text(31))),
        tax_registration: Some(Identifier(named::text(32))),
        additional_legal_information: Some(named::text(33)),
        electronic_address: Some(named_required_scheme(34)),
        address: named_address([35, 36, 162, 37, 38, 39, 40]),
        contact: Some(named_contact([41, 42, 43])),
    }
}

fn named_buyer() -> Buyer {
    Buyer {
        name: named::text(44),
        trading_name: Some(named::text(45)),
        identifier: Some(named_scheme(46)),
        legal_registration: Some(named_scheme(47)),
        vat_identifier: Some(Identifier(named::text(48))),
        electronic_address: Some(named_required_scheme(49)),
        address: named_address([50, 51, 163, 52, 53, 54, 55]),
        contact: Some(named_contact([56, 57, 58])),
    }
}

fn named_totals() -> DocumentTotals {
    DocumentTotals {
        line_net_total: named::amount(106),
        allowance_total: Some(named::amount(107)),
        charge_total: Some(named::amount(108)),
        total_without_vat: named::amount(109),
        vat_total: Some(named::amount(110)),
        vat_total_in_accounting_currency: Some(named::amount(111)),
        total_with_vat: named::amount(112),
        paid: Some(named::amount(113)),
        rounding: Some(named::amount(114)),
        due_for_payment: named::amount(115),
    }
}

/// The payment of the invoice that names its terms: the means, and one of
/// the three ways to pay, which exclude each other.
fn named_payment(kind: NamedPayment) -> PaymentInstructions {
    PaymentInstructions {
        means_code: Code(named::text(81)),
        means_text: Some(named::text(82)),
        remittance_information: Some(named::text(83)),
        credit_transfers: match kind {
            NamedPayment::CreditTransfer => vec![CreditTransfer {
                account: Identifier(named::text(84)),
                account_name: Some(named::text(85)),
                service_provider: Some(Identifier(named::text(86))),
            }],
            NamedPayment::Card | NamedPayment::DirectDebit => Vec::new(),
        },
        card: (kind == NamedPayment::Card).then(|| doc2einvoice_model::PaymentCard {
            primary_account_number: named::text(87),
            holder_name: Some(named::text(88)),
        }),
        direct_debit: (kind == NamedPayment::DirectDebit).then(|| DirectDebit {
            mandate_reference: Some(Identifier(named::text(89))),
            creditor_identifier: Some(Identifier(named::text(90))),
            debited_account: Some(Identifier(named::text(91))),
        }),
    }
}

/// How the invoice that names its terms is paid.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NamedPayment {
    CreditTransfer,
    Card,
    DirectDebit,
}

/// The attachment of the invoice that names its terms (BT-125): its content
/// is the text of the term.
pub fn named_attachment() -> Vec<u8> {
    named::text(125).into_bytes()
}

/// An invoice in which every business term carries a value of its own that
/// names the term, as [`named::names`] tells, paid in the given way. A
/// coded term carries a code no other term of its kind carries. The amounts
/// do not add up: the invoice is written, not validated.
///
/// The VAT point is a date (BT-7); [`named_with_vat_point_code`] gives the
/// code (BT-8).
pub fn named_invoice(kind: NamedPayment) -> Invoice {
    Invoice {
        number: Identifier(named::text(1)),
        issue_date: named::day(2),
        type_code: Code(NAMED_TYPE.to_owned()),
        currency: named_currency(5),
        vat_accounting_currency: Some(named_currency(6)),
        vat_point: Some(VatPoint::Date(named::day(7))),
        payment_due_date: Some(named::day(9)),
        buyer_reference: Some(named::text(10)),
        project_reference: Some(named::text(11)),
        contract_reference: Some(named::text(12)),
        purchase_order_reference: Some(named::text(13)),
        sales_order_reference: Some(named::text(14)),
        receiving_advice_reference: Some(named::text(15)),
        despatch_advice_reference: Some(named::text(16)),
        tender_or_lot_reference: Some(named::text(17)),
        invoiced_object: Some(named_scheme(18)),
        buyer_accounting_reference: Some(named::text(19)),
        payment_terms: Some(named::text(20)),
        notes: vec![InvoiceNote {
            subject_code: Some(Code(named::text(21))),
            text: named::text(22),
        }],
        process_control: ProcessControl {
            business_process: Some(named::text(23)),
            specification: Identifier(named::text(24)),
        },
        preceding_invoices: vec![PrecedingInvoice {
            reference: named::text(25),
            issue_date: Some(named::day(26)),
        }],
        seller: named_seller(),
        buyer: named_buyer(),
        payee: Some(Payee {
            name: named::text(59),
            identifier: Some(named_scheme(60)),
            legal_registration: Some(named_scheme(61)),
        }),
        tax_representative: Some(TaxRepresentative {
            name: named::text(62),
            vat_identifier: Identifier(named::text(63)),
            address: named_address([64, 65, 164, 66, 67, 68, 69]),
        }),
        delivery: Some(DeliveryInformation {
            party_name: Some(named::text(70)),
            location: Some(named_scheme(71)),
            actual_date: Some(named::day(72)),
            invoicing_period: Some(Period {
                start: Some(named::day(73)),
                end: Some(named::day(74)),
            }),
            address: Some(named_address([75, 76, 165, 77, 78, 79, 80])),
        }),
        payment_instructions: Some(named_payment(kind)),
        allowances: vec![named_adjustment([92, 93, 94, 95, 96, 97, 98])],
        charges: vec![named_adjustment([99, 100, 101, 102, 103, 104, 105])],
        totals: named_totals(),
        vat_breakdown: NonEmpty::one(VatBreakdown {
            taxable_amount: named::amount(116),
            tax_amount: named::amount(117),
            category: named_category(118),
            rate: Some(named::percentage(119)),
            exemption_reason: Some(named::text(120)),
            exemption_reason_code: Some(Code(named::text(121))),
        }),
        supporting_documents: vec![SupportingDocument {
            reference: named::text(122),
            description: Some(named::text(123)),
            external_location: Some(named::text(124)),
            attachment: Some(BinaryObject {
                content: named_attachment(),
                mime_code: named::text(125),
                filename: named::text(125),
            }),
        }],
        lines: NonEmpty::one(named_line()),
    }
}

/// The code of the VAT point of the invoice that names its terms (BT-8),
/// of UNTDID 2005: the date of the delivery.
pub const NAMED_VAT_POINT: &str = "35";

/// The invoice that names its terms with the VAT point as a code (BT-8)
/// instead of a date (BT-7); the two exclude each other.
pub fn named_with_vat_point_code() -> Invoice {
    let mut invoice = named_invoice(NamedPayment::CreditTransfer);
    invoice.vat_point = Some(VatPoint::Code(Code(NAMED_VAT_POINT.to_owned())));
    invoice
}
