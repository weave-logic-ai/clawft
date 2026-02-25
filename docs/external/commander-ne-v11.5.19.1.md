# Commander NE v11.5.19.1 Analysis
## From Live Install: Boat Shop Marine, LLC (Lower Keys, FL) — localhost\\NE2008 (Nov 14, 2025)

**Not** mobile marina/dock app. Full Windows DMS/ERP/POS for marine/powersports/OPE dealers by MIC Systems (Costa Mesa, CA; 1979). .NET/SQL Server (rewritten from DOS 2005). Sub ~$10-20/user/mo. Site: [commanderne.com](https://commanderne.com) (WP site w/ demos/pricing/support).

| Field | Detail |
|-------|--------|
| **Version** | v11.5.19.1 (captured; current v17.4) |
| **Tech** | .NET Framework, SQL Server |
| **Platform** | Windows desktop |
| **Markets** | Marine, Powersports, V-Twin, OPE, Golf Carts, Vermeer |

## Core Modules (Screenshot-Ref'd)

### 1. Sales/Invoicing (POS)
Counter sales/invoices (#0000000014). Bill/Ship To, terms/tax/project/discount/tech codes. Toolbar: Save/Post/Dup/Delete/Update/Print/Email/Cashdrawer/Checkout.  
*IMG_3184*

### 2. Purchase Orders & PO Pad
Vendors/items (Yamaha parts), payments, SO links. Detail/summary/filter.  
*IMG_3185-3189*

### 3. Stock Order Gen (Min/Max Reorder)
Auto PO from thresholds/OEM/sales filters/batch. Buy Back Orders.  
*IMG_3188*

### 4. Customer Mgmt (CRM)
Active/Inactive/Prospect. Addr/phone/email/co tabs: Gen/Pay/Hist/Video/Custom.  
*IMG_3190*

### 5. Items/Price File (Parts Inv)
OEM (e.g., Yamaha 6YC-83710-03-00), pricing tiers/bin/stock/supersession/UPC/img. 600+ vendors (Mercury wkly). Tabs: Gen/Detail/Ref/Notes/Custom/Sales Hist.  
*IMG_3191*

### 6. Unit/Boat Inventory
Serial/Hull/Engines/Year/Make/Model/cyls/fuel/hours/trailer/cust/cond/wt/loc. Status: Stocked/Sold/etc. Sticker gen. Tabs: Gen/Detail/Acc/RO/Notes/Custom/Chgs.  
*IMG_3192: YAM F150 Serial LCA6HPL100149*

### 7. OEM Vendor Mgmt
OEM codes/accts/price files/batch/exch rates/accts. Vendors: Yamaha/Evinrude/Honda/LandNSea/Lewis/Donovan (17 recs).  
*IMG_3193*

### 8. Repair Orders (Service)
RO #/cust/invoice/pay/bal/YMM/serial/void/docs. Filter/search.  
*IMG_3196*

### 9. RO Import/Scan
Barcode parts/labor from `C:\\Users\\...\\Temp\\NEImportScan`. Save/Merge/Delete.  
*IMG_3197*

### 10. Config (Tax/Ship Via)
Ship: UPS/FedEx/USPS/DHL accts/codes. Tax codes/eff dates.  
*IMG_3195*

## Web Recon (commanderne.com)
- Confirms markets/tech/pricing.
- Videos: Demos/training/F&I sales.
- Support: Setup/install/files/FAQ/year-end.
- Integrations: Credit card proc.

**Opps for ClawFT**: ODBC/SQL hooks for inv/PO/RO sync. Discord alerts (low stock/RO status). Plugin: `clawft-plugin-commander-dms`.

*Compiled by klaweaver @ WeaveLogic (2025). Screenshots pending upload.*