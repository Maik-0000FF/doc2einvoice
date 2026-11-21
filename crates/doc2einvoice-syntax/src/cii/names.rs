// SPDX-FileCopyrightText: Maik-0000FF
// SPDX-License-Identifier: GPL-3.0-or-later

//! The names UN/CEFACT Cross Industry Invoice D16B gives its namespaces,
//! elements, attributes and fixed values, each written once.

pub(super) const RSM_NAMESPACE: &str =
    "urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100";
pub(super) const RAM_NAMESPACE: &str =
    "urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100";
pub(super) const UDT_NAMESPACE: &str =
    "urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100";
pub(super) const QDT_NAMESPACE: &str = "urn:un:unece:uncefact:data:standard:QualifiedDataType:100";

pub(super) const XMLNS_RSM: &str = "xmlns:rsm";
pub(super) const XMLNS_RAM: &str = "xmlns:ram";
pub(super) const XMLNS_UDT: &str = "xmlns:udt";
pub(super) const XMLNS_QDT: &str = "xmlns:qdt";

// Document structure.
pub(super) const CROSS_INDUSTRY_INVOICE: &str = "rsm:CrossIndustryInvoice";
pub(super) const EXCHANGED_DOCUMENT_CONTEXT: &str = "rsm:ExchangedDocumentContext";
pub(super) const EXCHANGED_DOCUMENT: &str = "rsm:ExchangedDocument";
pub(super) const SUPPLY_CHAIN_TRADE_TRANSACTION: &str = "rsm:SupplyChainTradeTransaction";

// Attributes.
pub(super) const CURRENCY_ID: &str = "currencyID";
pub(super) const SCHEME_ID: &str = "schemeID";
pub(super) const UNIT_CODE: &str = "unitCode";
pub(super) const LIST_ID: &str = "listID";
pub(super) const LIST_VERSION_ID: &str = "listVersionID";
pub(super) const FORMAT: &str = "format";
pub(super) const MIME_CODE: &str = "mimeCode";
pub(super) const FILENAME: &str = "filename";

// Data types.
pub(super) const DATE_TIME_STRING: &str = "udt:DateTimeString";
pub(super) const DATE_STRING: &str = "udt:DateString";
pub(super) const QUALIFIED_DATE_TIME_STRING: &str = "qdt:DateTimeString";
pub(super) const INDICATOR: &str = "udt:Indicator";

// Elements.
pub(super) const ACTUAL_AMOUNT: &str = "ram:ActualAmount";
pub(super) const ACTUAL_DELIVERY_SUPPLY_CHAIN_EVENT: &str = "ram:ActualDeliverySupplyChainEvent";
pub(super) const ACCOUNT_NAME: &str = "ram:AccountName";
pub(super) const ADDITIONAL_REFERENCED_DOCUMENT: &str = "ram:AdditionalReferencedDocument";
pub(super) const ALLOWANCE_TOTAL_AMOUNT: &str = "ram:AllowanceTotalAmount";
pub(super) const APPLICABLE_HEADER_TRADE_AGREEMENT: &str = "ram:ApplicableHeaderTradeAgreement";
pub(super) const APPLICABLE_HEADER_TRADE_DELIVERY: &str = "ram:ApplicableHeaderTradeDelivery";
pub(super) const APPLICABLE_HEADER_TRADE_SETTLEMENT: &str = "ram:ApplicableHeaderTradeSettlement";
pub(super) const APPLICABLE_PRODUCT_CHARACTERISTIC: &str = "ram:ApplicableProductCharacteristic";
pub(super) const APPLICABLE_TRADE_SETTLEMENT_FINANCIAL_CARD: &str =
    "ram:ApplicableTradeSettlementFinancialCard";
pub(super) const APPLICABLE_TRADE_TAX: &str = "ram:ApplicableTradeTax";
pub(super) const APPLIED_TRADE_ALLOWANCE_CHARGE: &str = "ram:AppliedTradeAllowanceCharge";
pub(super) const ASSOCIATED_DOCUMENT_LINE_DOCUMENT: &str = "ram:AssociatedDocumentLineDocument";
pub(super) const ATTACHMENT_BINARY_OBJECT: &str = "ram:AttachmentBinaryObject";
pub(super) const BASIS_AMOUNT: &str = "ram:BasisAmount";
pub(super) const BASIS_QUANTITY: &str = "ram:BasisQuantity";
pub(super) const BIC_ID: &str = "ram:BICID";
pub(super) const BILLED_QUANTITY: &str = "ram:BilledQuantity";
pub(super) const BILLING_SPECIFIED_PERIOD: &str = "ram:BillingSpecifiedPeriod";
pub(super) const BUSINESS_PROCESS_PARAMETER: &str =
    "ram:BusinessProcessSpecifiedDocumentContextParameter";
pub(super) const BUYER_ASSIGNED_ID: &str = "ram:BuyerAssignedID";
pub(super) const BUYER_ORDER_REFERENCED_DOCUMENT: &str = "ram:BuyerOrderReferencedDocument";
pub(super) const BUYER_REFERENCE: &str = "ram:BuyerReference";
pub(super) const BUYER_TRADE_PARTY: &str = "ram:BuyerTradeParty";
pub(super) const CALCULATED_AMOUNT: &str = "ram:CalculatedAmount";
pub(super) const CALCULATION_PERCENT: &str = "ram:CalculationPercent";
pub(super) const CARDHOLDER_NAME: &str = "ram:CardholderName";
pub(super) const CATEGORY_CODE: &str = "ram:CategoryCode";
pub(super) const CATEGORY_TRADE_TAX: &str = "ram:CategoryTradeTax";
pub(super) const CHARGE_AMOUNT: &str = "ram:ChargeAmount";
pub(super) const CHARGE_INDICATOR: &str = "ram:ChargeIndicator";
pub(super) const CHARGE_TOTAL_AMOUNT: &str = "ram:ChargeTotalAmount";
pub(super) const CITY_NAME: &str = "ram:CityName";
pub(super) const CLASS_CODE: &str = "ram:ClassCode";
pub(super) const COMPLETE_NUMBER: &str = "ram:CompleteNumber";
pub(super) const CONTENT: &str = "ram:Content";
pub(super) const CONTRACT_REFERENCED_DOCUMENT: &str = "ram:ContractReferencedDocument";
pub(super) const COUNTRY_ID: &str = "ram:CountryID";
pub(super) const COUNTRY_SUB_DIVISION_NAME: &str = "ram:CountrySubDivisionName";
pub(super) const CREDITOR_REFERENCE_ID: &str = "ram:CreditorReferenceID";
pub(super) const DEFINED_TRADE_CONTACT: &str = "ram:DefinedTradeContact";
pub(super) const DESCRIPTION: &str = "ram:Description";
pub(super) const DESIGNATED_PRODUCT_CLASSIFICATION: &str = "ram:DesignatedProductClassification";
pub(super) const DESPATCH_ADVICE_REFERENCED_DOCUMENT: &str = "ram:DespatchAdviceReferencedDocument";
pub(super) const DIRECT_DEBIT_MANDATE_ID: &str = "ram:DirectDebitMandateID";
pub(super) const DUE_DATE_DATE_TIME: &str = "ram:DueDateDateTime";
pub(super) const DUE_DATE_TYPE_CODE: &str = "ram:DueDateTypeCode";
pub(super) const DUE_PAYABLE_AMOUNT: &str = "ram:DuePayableAmount";
pub(super) const EMAIL_URI_UNIVERSAL_COMMUNICATION: &str = "ram:EmailURIUniversalCommunication";
pub(super) const END_DATE_TIME: &str = "ram:EndDateTime";
pub(super) const EXEMPTION_REASON: &str = "ram:ExemptionReason";
pub(super) const EXEMPTION_REASON_CODE: &str = "ram:ExemptionReasonCode";
pub(super) const FORMATTED_ISSUE_DATE_TIME: &str = "ram:FormattedIssueDateTime";
pub(super) const GLOBAL_ID: &str = "ram:GlobalID";
pub(super) const GRAND_TOTAL_AMOUNT: &str = "ram:GrandTotalAmount";
pub(super) const GROSS_PRICE_PRODUCT_TRADE_PRICE: &str = "ram:GrossPriceProductTradePrice";
pub(super) const GUIDELINE_PARAMETER: &str = "ram:GuidelineSpecifiedDocumentContextParameter";
pub(super) const IBAN_ID: &str = "ram:IBANID";
pub(super) const ID: &str = "ram:ID";
pub(super) const INCLUDED_NOTE: &str = "ram:IncludedNote";
pub(super) const INCLUDED_SUPPLY_CHAIN_TRADE_LINE_ITEM: &str =
    "ram:IncludedSupplyChainTradeLineItem";
pub(super) const INFORMATION: &str = "ram:Information";
pub(super) const INVOICE_CURRENCY_CODE: &str = "ram:InvoiceCurrencyCode";
pub(super) const INVOICE_REFERENCED_DOCUMENT: &str = "ram:InvoiceReferencedDocument";
pub(super) const ISSUE_DATE_TIME: &str = "ram:IssueDateTime";
pub(super) const ISSUER_ASSIGNED_ID: &str = "ram:IssuerAssignedID";
pub(super) const LINE_ID: &str = "ram:LineID";
pub(super) const LINE_ONE: &str = "ram:LineOne";
pub(super) const LINE_THREE: &str = "ram:LineThree";
pub(super) const LINE_TOTAL_AMOUNT: &str = "ram:LineTotalAmount";
pub(super) const LINE_TWO: &str = "ram:LineTwo";
pub(super) const NAME: &str = "ram:Name";
pub(super) const NET_PRICE_PRODUCT_TRADE_PRICE: &str = "ram:NetPriceProductTradePrice";
pub(super) const OCCURRENCE_DATE_TIME: &str = "ram:OccurrenceDateTime";
pub(super) const ORIGIN_TRADE_COUNTRY: &str = "ram:OriginTradeCountry";
pub(super) const PAYEE_PARTY_CREDITOR_FINANCIAL_ACCOUNT: &str =
    "ram:PayeePartyCreditorFinancialAccount";
pub(super) const PAYEE_SPECIFIED_CREDITOR_FINANCIAL_INSTITUTION: &str =
    "ram:PayeeSpecifiedCreditorFinancialInstitution";
pub(super) const PAYEE_TRADE_PARTY: &str = "ram:PayeeTradeParty";
pub(super) const PAYER_PARTY_DEBTOR_FINANCIAL_ACCOUNT: &str =
    "ram:PayerPartyDebtorFinancialAccount";
pub(super) const PAYMENT_REFERENCE: &str = "ram:PaymentReference";
pub(super) const PERSON_NAME: &str = "ram:PersonName";
pub(super) const POSTAL_TRADE_ADDRESS: &str = "ram:PostalTradeAddress";
pub(super) const POSTCODE_CODE: &str = "ram:PostcodeCode";
pub(super) const PROPRIETARY_ID: &str = "ram:ProprietaryID";
pub(super) const RATE_APPLICABLE_PERCENT: &str = "ram:RateApplicablePercent";
pub(super) const REASON: &str = "ram:Reason";
pub(super) const REASON_CODE: &str = "ram:ReasonCode";
pub(super) const RECEIVABLE_ACCOUNTING_ACCOUNT: &str =
    "ram:ReceivableSpecifiedTradeAccountingAccount";
pub(super) const RECEIVING_ADVICE_REFERENCED_DOCUMENT: &str =
    "ram:ReceivingAdviceReferencedDocument";
pub(super) const REFERENCE_TYPE_CODE: &str = "ram:ReferenceTypeCode";
pub(super) const ROUNDING_AMOUNT: &str = "ram:RoundingAmount";
pub(super) const SELLER_ASSIGNED_ID: &str = "ram:SellerAssignedID";
pub(super) const SELLER_ORDER_REFERENCED_DOCUMENT: &str = "ram:SellerOrderReferencedDocument";
pub(super) const SELLER_TAX_REPRESENTATIVE_TRADE_PARTY: &str =
    "ram:SellerTaxRepresentativeTradeParty";
pub(super) const SELLER_TRADE_PARTY: &str = "ram:SellerTradeParty";
pub(super) const SHIP_TO_TRADE_PARTY: &str = "ram:ShipToTradeParty";
pub(super) const SPECIFIED_LEGAL_ORGANIZATION: &str = "ram:SpecifiedLegalOrganization";
pub(super) const SPECIFIED_LINE_TRADE_AGREEMENT: &str = "ram:SpecifiedLineTradeAgreement";
pub(super) const SPECIFIED_LINE_TRADE_DELIVERY: &str = "ram:SpecifiedLineTradeDelivery";
pub(super) const SPECIFIED_LINE_TRADE_SETTLEMENT: &str = "ram:SpecifiedLineTradeSettlement";
pub(super) const SPECIFIED_PROCURING_PROJECT: &str = "ram:SpecifiedProcuringProject";
pub(super) const SPECIFIED_TAX_REGISTRATION: &str = "ram:SpecifiedTaxRegistration";
pub(super) const SPECIFIED_TRADE_ALLOWANCE_CHARGE: &str = "ram:SpecifiedTradeAllowanceCharge";
pub(super) const SPECIFIED_TRADE_PAYMENT_TERMS: &str = "ram:SpecifiedTradePaymentTerms";
pub(super) const SPECIFIED_TRADE_PRODUCT: &str = "ram:SpecifiedTradeProduct";
pub(super) const SPECIFIED_TRADE_SETTLEMENT_HEADER_MONETARY_SUMMATION: &str =
    "ram:SpecifiedTradeSettlementHeaderMonetarySummation";
pub(super) const SPECIFIED_TRADE_SETTLEMENT_LINE_MONETARY_SUMMATION: &str =
    "ram:SpecifiedTradeSettlementLineMonetarySummation";
pub(super) const SPECIFIED_TRADE_SETTLEMENT_PAYMENT_MEANS: &str =
    "ram:SpecifiedTradeSettlementPaymentMeans";
pub(super) const START_DATE_TIME: &str = "ram:StartDateTime";
pub(super) const SUBJECT_CODE: &str = "ram:SubjectCode";
pub(super) const TAX_BASIS_TOTAL_AMOUNT: &str = "ram:TaxBasisTotalAmount";
pub(super) const TAX_CURRENCY_CODE: &str = "ram:TaxCurrencyCode";
pub(super) const TAX_POINT_DATE: &str = "ram:TaxPointDate";
pub(super) const TAX_TOTAL_AMOUNT: &str = "ram:TaxTotalAmount";
pub(super) const TELEPHONE_UNIVERSAL_COMMUNICATION: &str = "ram:TelephoneUniversalCommunication";
pub(super) const TOTAL_PREPAID_AMOUNT: &str = "ram:TotalPrepaidAmount";
pub(super) const TRADING_BUSINESS_NAME: &str = "ram:TradingBusinessName";
pub(super) const TYPE_CODE: &str = "ram:TypeCode";
pub(super) const URI_ID: &str = "ram:URIID";
pub(super) const URI_UNIVERSAL_COMMUNICATION: &str = "ram:URIUniversalCommunication";
pub(super) const VALUE: &str = "ram:Value";

// Fixed values.
/// The tax type of value added tax (UNTDID 5153).
pub(super) const VAT_TYPE: &str = "VAT";
/// The scheme of a VAT identifier in a tax registration.
pub(super) const VAT_REGISTRATION: &str = "VA";
/// The scheme of a local tax registration (BT-32).
pub(super) const LOCAL_TAX_REGISTRATION: &str = "FC";
/// The document type codes (UNTDID 1001) of the additional referenced
/// documents: a tender or lot (BT-17), an invoiced object (BT-18) and a
/// supporting document (BG-24).
pub(super) const TENDER_TYPE: &str = "50";
pub(super) const INVOICED_OBJECT_TYPE: &str = "130";
pub(super) const SUPPORTING_DOCUMENT_TYPE: &str = "916";
/// The name CII requires beside the identifier of a project (BT-11), which
/// no business term holds; written as the published examples of EN 16931 and
/// XRechnung write it.
pub(super) const PROJECT_NAME: &str = "Project reference";
/// The payment means (UNTDID 4461) of the SEPA area, whose accounts are
/// IBANs: credit transfer and direct debit.
pub(super) const SEPA_PAYMENT_MEANS: [&str; 2] = ["58", "59"];
pub(super) const TRUE: &str = "true";
pub(super) const FALSE: &str = "false";
