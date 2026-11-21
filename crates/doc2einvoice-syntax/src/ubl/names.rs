// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The names UBL 2.1 gives its namespaces, elements, attributes and fixed
//! values, each written once.

/// The namespace of an invoice document.
pub(super) const INVOICE_NAMESPACE: &str = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2";
/// The namespace of a credit note document.
pub(super) const CREDIT_NOTE_NAMESPACE: &str =
    "urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2";
/// The namespace of the aggregate components.
pub(super) const CAC_NAMESPACE: &str =
    "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2";
/// The namespace of the basic components.
pub(super) const CBC_NAMESPACE: &str =
    "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2";

pub(super) const XMLNS: &str = "xmlns";
pub(super) const XMLNS_CAC: &str = "xmlns:cac";
pub(super) const XMLNS_CBC: &str = "xmlns:cbc";

// Root elements.
pub(super) const INVOICE: &str = "Invoice";
pub(super) const CREDIT_NOTE: &str = "CreditNote";

// Attributes.
pub(super) const CURRENCY_ID: &str = "currencyID";
pub(super) const SCHEME_ID: &str = "schemeID";
pub(super) const UNIT_CODE: &str = "unitCode";
pub(super) const LIST_ID: &str = "listID";
pub(super) const LIST_VERSION_ID: &str = "listVersionID";
pub(super) const NAME: &str = "name";
pub(super) const MIME_CODE: &str = "mimeCode";
pub(super) const FILENAME: &str = "filename";

// Basic components.
pub(super) const ACCOUNTING_COST: &str = "cbc:AccountingCost";
pub(super) const ACTUAL_DELIVERY_DATE: &str = "cbc:ActualDeliveryDate";
pub(super) const ADDITIONAL_STREET_NAME: &str = "cbc:AdditionalStreetName";
pub(super) const ALLOWANCE_CHARGE_REASON: &str = "cbc:AllowanceChargeReason";
pub(super) const ALLOWANCE_CHARGE_REASON_CODE: &str = "cbc:AllowanceChargeReasonCode";
pub(super) const ALLOWANCE_TOTAL_AMOUNT: &str = "cbc:AllowanceTotalAmount";
pub(super) const AMOUNT: &str = "cbc:Amount";
pub(super) const BASE_AMOUNT: &str = "cbc:BaseAmount";
pub(super) const BASE_QUANTITY: &str = "cbc:BaseQuantity";
pub(super) const BUYER_REFERENCE: &str = "cbc:BuyerReference";
pub(super) const CHARGE_INDICATOR: &str = "cbc:ChargeIndicator";
pub(super) const CHARGE_TOTAL_AMOUNT: &str = "cbc:ChargeTotalAmount";
pub(super) const CITY_NAME: &str = "cbc:CityName";
pub(super) const COMPANY_ID: &str = "cbc:CompanyID";
pub(super) const COMPANY_LEGAL_FORM: &str = "cbc:CompanyLegalForm";
pub(super) const COUNTRY_SUBENTITY: &str = "cbc:CountrySubentity";
pub(super) const CREDIT_NOTE_TYPE_CODE: &str = "cbc:CreditNoteTypeCode";
pub(super) const CREDITED_QUANTITY: &str = "cbc:CreditedQuantity";
pub(super) const CUSTOMIZATION_ID: &str = "cbc:CustomizationID";
pub(super) const DESCRIPTION: &str = "cbc:Description";
pub(super) const DESCRIPTION_CODE: &str = "cbc:DescriptionCode";
pub(super) const DOCUMENT_CURRENCY_CODE: &str = "cbc:DocumentCurrencyCode";
pub(super) const DOCUMENT_DESCRIPTION: &str = "cbc:DocumentDescription";
pub(super) const DOCUMENT_TYPE_CODE: &str = "cbc:DocumentTypeCode";
pub(super) const DUE_DATE: &str = "cbc:DueDate";
pub(super) const ELECTRONIC_MAIL: &str = "cbc:ElectronicMail";
pub(super) const EMBEDDED_DOCUMENT_BINARY_OBJECT: &str = "cbc:EmbeddedDocumentBinaryObject";
pub(super) const END_DATE: &str = "cbc:EndDate";
pub(super) const ENDPOINT_ID: &str = "cbc:EndpointID";
pub(super) const ID: &str = "cbc:ID";
pub(super) const IDENTIFICATION_CODE: &str = "cbc:IdentificationCode";
pub(super) const INVOICE_TYPE_CODE: &str = "cbc:InvoiceTypeCode";
pub(super) const INVOICED_QUANTITY: &str = "cbc:InvoicedQuantity";
pub(super) const ISSUE_DATE: &str = "cbc:IssueDate";
pub(super) const ITEM_CLASSIFICATION_CODE: &str = "cbc:ItemClassificationCode";
pub(super) const LINE: &str = "cbc:Line";
pub(super) const LINE_EXTENSION_AMOUNT: &str = "cbc:LineExtensionAmount";
pub(super) const LINE_ID: &str = "cbc:LineID";
pub(super) const MULTIPLIER_FACTOR_NUMERIC: &str = "cbc:MultiplierFactorNumeric";
pub(super) const NAME_ELEMENT: &str = "cbc:Name";
pub(super) const NOTE: &str = "cbc:Note";
pub(super) const PAYABLE_AMOUNT: &str = "cbc:PayableAmount";
pub(super) const PAYABLE_ROUNDING_AMOUNT: &str = "cbc:PayableRoundingAmount";
pub(super) const PAYMENT_DUE_DATE: &str = "cbc:PaymentDueDate";
pub(super) const PAYMENT_ID: &str = "cbc:PaymentID";
pub(super) const PAYMENT_MEANS_CODE: &str = "cbc:PaymentMeansCode";
pub(super) const PERCENT: &str = "cbc:Percent";
pub(super) const POSTAL_ZONE: &str = "cbc:PostalZone";
pub(super) const PREPAID_AMOUNT: &str = "cbc:PrepaidAmount";
pub(super) const PRICE_AMOUNT: &str = "cbc:PriceAmount";
pub(super) const PROFILE_ID: &str = "cbc:ProfileID";
pub(super) const REGISTRATION_NAME: &str = "cbc:RegistrationName";
pub(super) const SALES_ORDER_ID: &str = "cbc:SalesOrderID";
pub(super) const START_DATE: &str = "cbc:StartDate";
pub(super) const STREET_NAME: &str = "cbc:StreetName";
pub(super) const TAX_AMOUNT: &str = "cbc:TaxAmount";
pub(super) const TAX_CURRENCY_CODE: &str = "cbc:TaxCurrencyCode";
pub(super) const TAX_EXCLUSIVE_AMOUNT: &str = "cbc:TaxExclusiveAmount";
pub(super) const TAX_EXEMPTION_REASON: &str = "cbc:TaxExemptionReason";
pub(super) const TAX_EXEMPTION_REASON_CODE: &str = "cbc:TaxExemptionReasonCode";
pub(super) const TAX_INCLUSIVE_AMOUNT: &str = "cbc:TaxInclusiveAmount";
pub(super) const TAX_POINT_DATE: &str = "cbc:TaxPointDate";
pub(super) const TAXABLE_AMOUNT: &str = "cbc:TaxableAmount";
pub(super) const TELEPHONE: &str = "cbc:Telephone";
pub(super) const URI: &str = "cbc:URI";
pub(super) const VALUE: &str = "cbc:Value";

// Aggregate components.
pub(super) const ACCOUNTING_CUSTOMER_PARTY: &str = "cac:AccountingCustomerParty";
pub(super) const ACCOUNTING_SUPPLIER_PARTY: &str = "cac:AccountingSupplierParty";
pub(super) const ADDITIONAL_DOCUMENT_REFERENCE: &str = "cac:AdditionalDocumentReference";
pub(super) const ADDITIONAL_ITEM_PROPERTY: &str = "cac:AdditionalItemProperty";
pub(super) const ADDRESS: &str = "cac:Address";
pub(super) const ADDRESS_LINE: &str = "cac:AddressLine";
pub(super) const ALLOWANCE_CHARGE: &str = "cac:AllowanceCharge";
pub(super) const ATTACHMENT: &str = "cac:Attachment";
pub(super) const BILLING_REFERENCE: &str = "cac:BillingReference";
pub(super) const BUYERS_ITEM_IDENTIFICATION: &str = "cac:BuyersItemIdentification";
pub(super) const CLASSIFIED_TAX_CATEGORY: &str = "cac:ClassifiedTaxCategory";
pub(super) const COMMODITY_CLASSIFICATION: &str = "cac:CommodityClassification";
pub(super) const CONTACT: &str = "cac:Contact";
pub(super) const CONTRACT_DOCUMENT_REFERENCE: &str = "cac:ContractDocumentReference";
pub(super) const COUNTRY: &str = "cac:Country";
pub(super) const CREDIT_NOTE_LINE: &str = "cac:CreditNoteLine";
pub(super) const DELIVERY: &str = "cac:Delivery";
pub(super) const DELIVERY_LOCATION: &str = "cac:DeliveryLocation";
pub(super) const DELIVERY_PARTY: &str = "cac:DeliveryParty";
pub(super) const DESPATCH_DOCUMENT_REFERENCE: &str = "cac:DespatchDocumentReference";
pub(super) const DOCUMENT_REFERENCE: &str = "cac:DocumentReference";
pub(super) const EXTERNAL_REFERENCE: &str = "cac:ExternalReference";
pub(super) const FINANCIAL_INSTITUTION_BRANCH: &str = "cac:FinancialInstitutionBranch";
pub(super) const INVOICE_DOCUMENT_REFERENCE: &str = "cac:InvoiceDocumentReference";
pub(super) const INVOICE_LINE: &str = "cac:InvoiceLine";
pub(super) const INVOICE_PERIOD: &str = "cac:InvoicePeriod";
pub(super) const ITEM: &str = "cac:Item";
pub(super) const LEGAL_MONETARY_TOTAL: &str = "cac:LegalMonetaryTotal";
pub(super) const ORDER_LINE_REFERENCE: &str = "cac:OrderLineReference";
pub(super) const ORDER_REFERENCE: &str = "cac:OrderReference";
pub(super) const ORIGIN_COUNTRY: &str = "cac:OriginCountry";
pub(super) const ORIGINATOR_DOCUMENT_REFERENCE: &str = "cac:OriginatorDocumentReference";
pub(super) const PARTY: &str = "cac:Party";
pub(super) const PARTY_IDENTIFICATION: &str = "cac:PartyIdentification";
pub(super) const PARTY_LEGAL_ENTITY: &str = "cac:PartyLegalEntity";
pub(super) const PARTY_NAME: &str = "cac:PartyName";
pub(super) const PARTY_TAX_SCHEME: &str = "cac:PartyTaxScheme";
pub(super) const PAYEE_FINANCIAL_ACCOUNT: &str = "cac:PayeeFinancialAccount";
pub(super) const PAYEE_PARTY: &str = "cac:PayeeParty";
pub(super) const PAYER_FINANCIAL_ACCOUNT: &str = "cac:PayerFinancialAccount";
pub(super) const PAYMENT_MANDATE: &str = "cac:PaymentMandate";
pub(super) const PAYMENT_MEANS: &str = "cac:PaymentMeans";
pub(super) const PAYMENT_TERMS: &str = "cac:PaymentTerms";
pub(super) const POSTAL_ADDRESS: &str = "cac:PostalAddress";
pub(super) const PRICE: &str = "cac:Price";
pub(super) const PROJECT_REFERENCE: &str = "cac:ProjectReference";
pub(super) const RECEIPT_DOCUMENT_REFERENCE: &str = "cac:ReceiptDocumentReference";
pub(super) const SELLERS_ITEM_IDENTIFICATION: &str = "cac:SellersItemIdentification";
pub(super) const STANDARD_ITEM_IDENTIFICATION: &str = "cac:StandardItemIdentification";
pub(super) const TAX_CATEGORY: &str = "cac:TaxCategory";
pub(super) const TAX_REPRESENTATIVE_PARTY: &str = "cac:TaxRepresentativeParty";
pub(super) const TAX_SCHEME: &str = "cac:TaxScheme";
pub(super) const TAX_SUBTOTAL: &str = "cac:TaxSubtotal";
pub(super) const TAX_TOTAL: &str = "cac:TaxTotal";

// Fixed values.
/// The tax scheme of value added tax.
pub(super) const VAT_SCHEME: &str = "VAT";
/// The tax scheme of a local tax registration (BT-32), as the published
/// examples of EN 16931 and XRechnung write it.
pub(super) const LOCAL_TAX_SCHEME: &str = "FC";
/// The identification scheme of the SEPA creditor identifier (BT-90).
pub(super) const SEPA_SCHEME: &str = "SEPA";
/// The document type code (UNTDID 1001) of an invoiced object (BT-18,
/// BT-128).
pub(super) const INVOICED_OBJECT_TYPE: &str = "130";
/// The document type code (UNTDID 1001) of a project reference in a credit
/// note (BT-11), which has no project reference element.
pub(super) const PROJECT_TYPE: &str = "50";
pub(super) const TRUE: &str = "true";
pub(super) const FALSE: &str = "false";
/// Separates the subject code of a note (BT-21) from its text, as in
/// `#AAI#Text`.
pub(super) const NOTE_SUBJECT_DELIMITER: &str = "#";
