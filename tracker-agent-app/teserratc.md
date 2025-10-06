Nápad přesunout celou logiku zpracování obrazu a OCR přímo na klientské PC, kde je aplikace spuštěna, je z pohledu architektury velmi strategický a správný. Tím, že Rust klientská aplikace zpracuje snímek obrazovky pomocí Tesseractu a odešle na server pouze hotový, strukturovaný text, dosáhnete tří klíčových výhod:

Odlehčení Serveru (Zero-Load Server): Server je zbaven veškeré výpočetně náročné práce (OCR je náročné na CPU) a místo zpracování obrazu se zaměří pouze na zpracování dat (analýza textu, uložení, distribuce).

Okamžitá Rychlost Odečtu: Latence (doba od pořízení screenu po získání textu) bude minimální, protože nedochází k přenosu velkého obrazového souboru přes síť. I přes to, že Tesseract je CPU náročný, moderní desktopové CPU (Mac, Win, Linux) jej zvládnou zpracovat rychleji, než by trval síťový přenos a následná fronta na serveru.

Minimalizace Datového Toku: Místo odesílání velkých obrazových dat (např. 1–5 MB na snímek) odesíláte malý balíček textu (několik KB), což zlepšuje spolehlivost i při horším internetovém připojení.

Níže uvádím, jak by tato nová architektura pro klientskou Rust aplikaci mohla vypadat a jak řešit technické výzvy.

Architektonická Změna: Migrace OCR do Rust Klienta (On-Device Processing)
I. Implementace Tesseractu v Rust Klientovi
Pro klientskou aplikaci napsanou v Rustu je klíčové použít vazbu Tesseractu, která se dokáže zkompilovat pro všechny tři cílové platformy (Windows, macOS, Linux), což vazba tesseract-rs s funkcí build-tesseract podporuje.

1. Integrace a Závislosti
Protože Tesseract je C++ knihovna, je nutné ji zkompilovat spolu s vaší Rust aplikací.

Architektonická Součást	Knihovna (Crate) v Rustu
Vazba Tesseractu	
tesseract-rs (s povolenou funkcí build-tesseract)    

Zpracování obrazu	image, imageproc
Základní OCR Workflow	Synchronní volání Tesseract API
Poznámka ke Kompilaci: Jelikož vaše aplikace již úspěšně kompiluje pro Mac, Windows a Linux, a je již schopna řešit problémy s cross-kompilací, přidání Tesseractu (který vyžaduje kompilátor C++ a CMake) do procesu kompilace je další, zvládnutelný krok. Je nutné zajistit, že výsledný distribuční balíček (např. instalátor) bude obsahovat všechny potřebné dynamické knihovny a také jazyková data Tesseractu (.traineddata).

2. Typický Klientský Workflow (Synchronní)
Klient by pracoval v tomto sekvenčním, synchronním cyklu:

Snímek: Pořízení screenu obrazovky.

Předzpracování obrazu: Konverze snímku do formátu optimalizovaného pro OCR (Grayscale, Binarizace) pomocí image a imageproc.

OCR: Blokující, CPU-náročné spuštění Tesseractu na předzpracovaném obrazu.

Extrakce dat: Získání textu a případných metadat (bounding boxy, spolehlivost).   

Odeslání dat: Odeslání výsledného, malého textového řetězce (JSON/Text) na server.

II. Optimalizace Výkonu (Latence a CPU Zátěž)
Přesunutí OCR na klienta ušetří server, ale přenese zátěž na uživatelský počítač. Vzhledem k tomu, že OCR je operace náročná na CPU (může trvat i více než 3 sekundy na obrázek ), je klíčové minimalizovat dopad na uživatelský zážitek.   

2.1. Řešení CPU Zátěže
Tesseract je navržen tak, aby maximálně využíval dostupný CPU. Na moderních procesorech (např. Intel Core i7-10750H) může být rychlost zpracování s optimalizacemi kolem 1,96–2,35 sekundy na stránku.   

Akční doporučení:

Asynchronní Spouštění: Ačkoliv se nejedná o asynchronní serverový runtime (jako Tokio), celá OCR operace musí být v klientské aplikaci spuštěna v samostatném vláknu na pozadí. Tím se zajistí, že hlavní vlákno GUI (pokud ho aplikace má) zůstane responsivní a uživatelské rozhraní se „nezasekne“. Jakmile je text zpracován, vlákno se připojí zpět a odešle výsledek.   

Optimalizace Vstupu: Využijte knihovny image a imageproc k zmenšení velikosti snímku a jeho konverzi do Grayscale a binarizaci. Menší obrázky se zpracovávají rychleji.   

2.2. Záměrné Vypuštění Korekce Šikmosti (Deskewing)
Vaše úvaha, že korekce šikmosti (deskewing) nemusí být pro snímky obrazovky okamžitě nutná, je architektonicky opodstatněná:

Pravděpodobnost Skosení: Snímky obrazovky (při použití standardních API) jsou téměř vždy dokonale zarovnané a vodorovné, pokud uživatel nemění rozlišení nebo orientaci obrazovky.

Vysoká Složitost: Funkce Deskewing vyžaduje pokročilé knihovny počítačového vidění, jako je OpenCV, a jejich integrace do cross-platformního balíku Rustu je velmi komplexní a přidává značnou velikost binárního souboru.   

Strategické Doporučení:
Začněte s čistým OCR na předpokladu zarovnaného vstupu. Tím eliminujete komplexní závislosti na OpenCV. Funkci Deskewing implementujte až v druhé fázi jako volitelný doplněk, pouze pokud testy prokážou, že přesnost rozpoznávání klesá v důsledku šikmých nebo hlučných snímků.   

III. Zajištění Přesnosti (Preprocessing Pipeline)
Tesseract dosahuje vysoké přesnosti (94–96 % na čistých dokumentech) , ale vyžaduje čistý vstup. Proto je nutné zavést základní pipeline pro úpravu snímku v Rustu:   

Krok	Funkce	Rust knihovna (image/imageproc)
1. Konverze do Grayscale	Redukce barevné složitosti na stupně šedi.	
image::DynamicImage::to_luma8()    

2. Binarizace (Thresholding)	Převod na čistě černobílý obraz, maximalizace kontrastu textu/pozadí.	
imageproc::contrast::threshold    

3. Odstranění Šumu (Despeckling)	Odstranění drobných artefaktů nebo šumu vzniklého kompresí.	
imageproc::filter::median_filter    

4. Konfigurace Tesseractu	
Nastavení Page Segmentation Mode (PSM) podle charakteru textu na obrazovce (např. PSM 7 pro jeden řádek nebo PSM 6 pro blok textu).   

Tento minimalistický, ale nezbytný pipeline by měl běžet na klientském PC ještě před voláním Tesseract::get_text().

IV. Srovnání s LLM (Architektonický Důsledek)
Kritérium	Původní Architektura (LLM na Serveru)	Nová Architektura (Tesseract na Klientovi)
Latence/Rychlost	
Pomalejší: Závisí na síti a frontě serveru (desítky sekund).   

Rychlejší: Zpracování přímo na CPU klienta (jednotky sekund).   

Zátěž Serveru	Vysoká: CPU a VRAM zátěž pro běh a analýzu LLM.	Nízká/Nulová: Server přijímá jen textový výstup, ne obrázky.
Náklady	
Vysoké/Variabilní: Poplatky za tokeny LLM (GPT-4o) nebo vysoké provozní náklady na vlastní GPU.   

Nulové Marginální Náklady: Jednorázové náklady na vývoj, nulové transakční náklady.
Přesnost (Text)	
Vynikající: LLM má kontext, zvládá složité layouty a ručně psaný text.   

Velmi Dobrá: Na snímcích obrazovky s jasným fontem a binarizací.   

Implementační Složitost	
Nízká (klient) Vysoká (server): Správa škálovatelnosti Rust serveru s blokujícími operacemi.   

Vysoká (klient): Správa FFI (C++) závislostí v cross-platformním balíčku.
Tato migrace vám umožní dosáhnout extrémní rychlosti a škálovatelnosti, které byste s centralizovaným LLM modelem pro OCR nedosáhli, a zároveň snížíte celkové provozní náklady na nulu. Server bude nadále sloužit pro sémantickou analýzu nebo ukládání, ale bude pracovat s textem, nikoli s obrazem.