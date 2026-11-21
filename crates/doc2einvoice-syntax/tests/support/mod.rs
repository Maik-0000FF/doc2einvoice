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
