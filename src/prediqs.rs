#![no_std]

multiversx_sc::imports!();

use multiversx_sc::proxy_imports::{NestedDecode, NestedEncode, TopDecode, TopEncode};
use multiversx_sc::derive_imports::type_abi;
use multiversx_sc::codec;
/// ## Prediqs Akıllı Sözleşmesi
/// Bu sözleşme, kullanıcıların çeşitli olayların sonuçları üzerine EGLD ile bahis oynamasına
/// olanak tanıyan bir tahmin piyasası platformudur.
///
/// ### Temel Özellikler:
/// - Sahip (Owner) tarafından yeni piyasalar (market) oluşturulabilir.
/// - Kullanıcılar, açık olan piyasalardaki olası sonuçlara EGLD yatırarak bahis oynayabilir.
/// - Piyasalar, yine Sahip tarafından "çözülür" (resolve), yani kazanan sonuç belirlenir.
/// - Kazanan bahisçiler, havuzdan kendi bahis oranlarına göre paylarını alırlar.
/// - Platform, her piyasanın toplam havuzundan %2 komisyon alır.

// --- Veri Yapıları (Structs) ---
// Bir piyasanın durumunu belirtmek için kullanılır.
#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy, PartialEq, Eq)]
pub enum MarketStatus {
    Open,      // Piyasa bahis almaya açık
    Resolved,  // Piyasa sonuçlandı, ödemeler beklenebilir
    Canceled,  // Piyasa iptal edildi
}

// V3 CHANGE: Kategori yapısı
#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct Category<M: ManagedTypeApi> {
    pub id: u64,
    pub name: ManagedBuffer<M>,
    pub active: bool,
}

// Her bir bahis piyasasının bilgilerini tutan ana yapı.
#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct Market<M: ManagedTypeApi> {
    pub id: u64,
    pub question: ManagedBuffer<M>, // Örn: "Bu akşamki maçı kim kazanır?"
    pub category_id: u64, // V3 CHANGE: Kategori ID'si
    pub outcomes: ManagedVec<M, ManagedBuffer<M>>, // Örn: ["A Takımı", "B Takımı", "Berabere"]
    pub creation_timestamp: u64,
    pub resolve_timestamp: u64, // Piyasının ne zaman sonuçlanması beklendiği
    pub betting_deadline: u64, // V2 CHANGE: Bahis bitiş zamanı
    pub status: MarketStatus,
    pub winning_outcome_index: Option<usize>, // Sonuçlandığında kazanan sonucun indeksi
    pub total_egld_pool: BigUint<M>, // Bu piyasada toplanan toplam EGLD miktarı
}

// YENİ: market_created_event için veri paketi struct'ı
#[type_abi]
#[derive(TopEncode, TopDecode)]
pub struct MarketCreatedEventData<M: ManagedTypeApi> {
    pub question: ManagedBuffer<M>,
    pub outcomes: ManagedVec<M, ManagedBuffer<M>>,
}

// --- Sözleşme Ana Bloğu ---
#[multiversx_sc::contract]
pub trait PrediqsContract {

    // --- Olaylar (Events) ---
    // Blokzincir üzerinde takip edilebilir loglar oluşturur.
    // GÜNCELLENDİ: Event tanımı artık tek bir veri argümanı (yeni struct) alıyor.
    #[event("market_created")]
    fn market_created_event(
        &self,
        #[indexed] market_id: u64,
        data: &MarketCreatedEventData<Self::Api>, // Tek veri argümanı
    );

    #[event("bet_placed")]
    fn bet_placed_event(
        &self,
        #[indexed] bettor: &ManagedAddress,
        #[indexed] market_id: u64,
        #[indexed] outcome_index: usize,
        amount: &BigUint,
    );

    #[event("market_resolved")]
    fn market_resolved_event(
        &self,
        #[indexed] market_id: u64,
        #[indexed] winning_outcome_index: usize,
    );

    #[event("winnings_claimed")]
    fn winnings_claimed_event(
        &self,
        #[indexed] claimant: &ManagedAddress,
        #[indexed] market_id: u64,
        amount: &BigUint,
    );

    


    // --- Depolama (Storage) ---
    // Blokzincir üzerinde kalıcı olarak veri saklamak için kullanılır.
    #[storage_mapper("markets")]
    fn markets(&self) -> VecMapper<Market<Self::Api>>;

    #[storage_mapper("bets")]
    fn bets(&self, market_id: u64, outcome_index: usize) -> MapMapper<ManagedAddress, BigUint>;

    #[storage_mapper("totalBetPerOutcome")]
    fn total_bet_per_outcome(&self, market_id: u64, outcome_index: usize) -> SingleValueMapper<BigUint>;

    #[storage_mapper("claimedWinnings")]
    fn claimed_winnings(&self, market_id: u64, user: &ManagedAddress) -> SingleValueMapper<bool>;
    
    // V2 CHANGE: İade işlemleri için yeni mapper
    #[storage_mapper("betsInMarketByUser")]
    fn bets_in_market_by_user(&self, user: &ManagedAddress, market_id: u64) -> SingleValueMapper<BigUint>;

    #[storage_mapper("claimedRefund")]
    fn claimed_refund(&self, market_id: u64, user: &ManagedAddress) -> SingleValueMapper<bool>;

    // V3 CHANGE: Kategori mapper'ı
    #[storage_mapper("categories")]
    fn categories(&self) -> VecMapper<Category<Self::Api>>;


    // --- Kurulum (Initialization) Fonksiyonu ---
    // Sözleşme ilk kez deploy edildiğinde sadece bir kere çalışır.
    #[init]
    fn init(&self) {
        // Bu fonksiyon, sözleşme sahibini otomatik olarak deploy eden kişi yapar.
    }

    #[upgrade]
    fn upgrade(&self) {
        // Bu fonksiyon, sözleşme sahibini otomatik olarak upgrade eden kişi yapar.
    }

    // --- Yönetici (Owner) Fonksiyonları ---

    /// Yeni bir tahmin piyasası oluşturur.
    /// Sadece sözleşme sahibi tarafından çağrılabilir.
     
    // V3 CHANGE: New owner-only functions to manage categories
    #[only_owner]
    #[endpoint(addCategory)]
    fn add_category(&self, name: ManagedBuffer) {
        let category_id = self.categories().len() as u64 + 1;
        let category = Category {
            id: category_id,
            name,
            active: true,
        };
        self.categories().push(&category);
    }

    #[only_owner]
    #[endpoint(toggleCategoryStatus)]
    fn toggle_category_status(&self, category_id: u64, active: bool) {
        require!(category_id > 0 && category_id as usize <= self.categories().len(), "Invalid category ID");
        let mut category = self.categories().get(category_id as usize);
        category.active = active;
        self.categories().set(category_id as usize, &category);
    }

    #[only_owner]
    #[endpoint(createMarket)]
    fn create_market(&self, question: ManagedBuffer, betting_deadline: u64, category_id: u64, outcomes: MultiValueEncoded<ManagedBuffer>) {
        let market_id = self.markets().len() as u64 + 1;
        let timestamp = self.blockchain().get_block_timestamp();
        let outcomes_vec = outcomes.to_vec();
        require!(betting_deadline > timestamp, "Betting deadline must be in the future");

        // V3 CHANGE: Validate the category ID
        require!(category_id > 0 && category_id as usize <= self.categories().len(), "Invalid category ID");
        let category = self.categories().get(category_id as usize);
        require!(category.active, "Category is not active");

        let market = Market {
            id: market_id,
            question: question.clone(),
            category_id,
            // outcomes.into_vec() yerine outcomes.to_vec() kullanıyoruz
            outcomes: outcomes_vec.clone(),
            creation_timestamp: timestamp,
            betting_deadline,
            resolve_timestamp: 0,
            status: MarketStatus::Open,
            winning_outcome_index: None,
            total_egld_pool: BigUint::zero(),
        };

        self.markets().push(&market);

        // GÜNCELLENDİ: Event'i çağırırken yeni struct'ı kullanıyoruz.
        let event_data = MarketCreatedEventData {
            question: market.question,
            outcomes: market.outcomes,
        };
        self.market_created_event(market_id, &event_data);
    }


    // V2 CHANGE: Piyasayı iptal etmek için yeni fonksiyon
    #[only_owner]
    #[endpoint(cancelMarket)]
    fn cancel_market(&self, market_id: u64) {
        let mut market = self.markets().get(market_id as usize);
        require!(market.status == MarketStatus::Open, "Market must be open to be canceled");
        market.status = MarketStatus::Canceled;
        self.markets().set(market_id as usize, &market);
    }

    /// Bir piyasanın sonucunu belirler.
    /// Sadece sözleşme sahibi (bizim Oracle'ımız) tarafından çağrılabilir.
    #[only_owner]
    #[endpoint(resolveMarket)]
    fn resolve_market(&self, market_id: u64, winning_outcome_index: usize) {
        require!(
            !self.markets().is_empty() && market_id as usize <= self.markets().len(),
            "Market ID does not exist"
        );

        let mut market = self.markets().get(market_id as usize);
        require!(market.status == MarketStatus::Open, "Market is not open for resolution");
        require!(winning_outcome_index < market.outcomes.len(), "Invalid winning outcome index");

        market.status = MarketStatus::Resolved;
        market.winning_outcome_index = Some(winning_outcome_index);

        self.markets().set(market_id as usize, &market);
        self.market_resolved_event(market_id, winning_outcome_index);
    }


    // --- Kullanıcı Fonksiyonları ---

    /// Açık bir piyasaya bahis oynamak için kullanılır.
    /// Bu fonksiyon EGLD kabul eder (`payable`).
    #[payable("EGLD")]
    #[endpoint(placeBet)]
    fn place_bet(&self, market_id: u64, outcome_index: usize) {
        let payment = self.call_value().egld();
        require!(*payment > 0, "Payment must be more than zero");

        let mut market = self.markets().get(market_id as usize);
        let timestamp = self.blockchain().get_block_timestamp();

        require!(market.status == MarketStatus::Open, "Market is not open for betting");
        require!(timestamp < market.betting_deadline, "Betting deadline has passed");
        require!(outcome_index < market.outcomes.len(), "Invalid outcome index");

        let caller = self.blockchain().get_caller();
        
        let mut current_bet = self.bets(market_id, outcome_index).get(&caller).unwrap_or_default();
        current_bet += &*payment;
        self.bets(market_id, outcome_index).insert(caller.clone(), current_bet);

        self.bets_in_market_by_user(&caller, market_id).update(|total| *total += &*payment);

        let mut total = self.total_bet_per_outcome(market_id, outcome_index).get();
        total += &*payment;
        self.total_bet_per_outcome(market_id, outcome_index).set(total);

        market.total_egld_pool += &*payment;
        self.markets().set(market_id as usize, &market);

        self.bet_placed_event(&caller, market_id, outcome_index, &payment);
    }

    /// Sonuçlanmış bir piyasadan kazancını talep eder.
    #[endpoint(claimWinnings)]
    fn claim_winnings(&self, market_id: u64) {
        let caller = self.blockchain().get_caller();
        require!(
            self.claimed_winnings(market_id, &caller).is_empty(),
            "Winnings already claimed for this market"
        );

        let market = self.markets().get(market_id as usize);
        require!(market.status == MarketStatus::Resolved, "Market is not resolved yet");

        let winning_outcome = market.winning_outcome_index.unwrap();
        
        // Kullanıcının kazanan sonuca ne kadar yatırdığını al
        let user_bet_on_winner = self.bets(market_id, winning_outcome).get(&caller).unwrap_or_default();
        require!(user_bet_on_winner > 0, "You did not bet on the winning outcome");

        // Kazanan havuzun toplam miktarını al
        let total_winning_pool = self.total_bet_per_outcome(market_id, winning_outcome).get();

        // Platform komisyonunu hesapla (%2)
        let total_pool = &market.total_egld_pool;
        let fee = total_pool * 2u64 / 100u64;
        let owner = self.blockchain().get_owner_address();
        self.send().direct_egld(&owner, &fee);

        // Dağıtılacak net havuz
        let distributable_pool = total_pool - &fee;

        // Kullanıcının kazancını hesapla (orantısal)
        // Payout = (Kullanıcının Kazanan Bahsi / Kazanan Havuzun Toplamı) * Dağıtılacak Net Havuz
        let payout = &user_bet_on_winner * &distributable_pool / &total_winning_pool;

        require!(payout > 0, "Payout amount is zero");

        self.send().direct_egld(&caller, &payout);
        self.claimed_winnings(market_id, &caller).set(true); // Kazancını aldığını işaretle
        self.winnings_claimed_event(&caller, market_id, &payout);
    }

    // V2 CHANGE: İptal edilen piyasadan iade almak için yeni fonksiyon
    #[endpoint(claimRefund)]
    fn claim_refund(&self, market_id: u64) {
        let caller = self.blockchain().get_caller();
        require!(self.claimed_refund(market_id, &caller).is_empty(), "Refund already claimed");
        let market = self.markets().get(market_id as usize);
        require!(market.status == MarketStatus::Canceled, "Market is not canceled");

        let total_user_bet = self.bets_in_market_by_user(&caller, market_id).get();
        require!(total_user_bet > 0, "You did not place any bets in this market");

        self.send().direct_egld(&caller, &total_user_bet);
        self.claimed_refund(market_id, &caller).set(true);
    }

    #[view(getMarket)]
    fn get_market(&self, market_id: u64) -> Market<Self::Api> {
        self.markets().get(market_id as usize)
    }

    // V3 CHANGE: New view function to get all categories
    #[view(getAllCategories)]
    fn get_all_categories(&self) -> MultiValueEncoded<Category<Self::Api>> {
        self.categories().iter().collect()
    }
}