use std::collections::HashMap;
use std::error::Error;
use std::{fmt, time};
use std::thread::sleep;

// Custom error type for RAG operations
#[derive(Debug)]
pub enum RagError {
    ApiError(String),
    EmbeddingError(String),
    InvalidInput(String),
}

impl fmt::Display for RagError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RagError::ApiError(msg) => write!(f, "API Error: {}", msg),
            RagError::EmbeddingError(msg) => write!(f, "Embedding Error: {}", msg),
            RagError::InvalidInput(msg) => write!(f, "Invalid Input: {}", msg),
        }
    }
}

impl Error for RagError {}

// Document structure to store text and its embedding
#[derive(Debug, Clone)]
pub struct Document {
    pub id: String,
    pub text: String,
    pub embedding: Vec<f32>,
    pub metadata: HashMap<String, String>,
}

// Similarity score with document reference
#[derive(Debug, Clone)]
pub struct SimilarityResult {
    pub document: Document,
    pub score: f32,
}

// In-memory vector store
pub struct VectorStore {
    documents: Vec<Document>,
    dimension: usize,
}

impl VectorStore {
    pub fn new(dimension: usize) -> Self {
        Self {
            documents: Vec::new(),
            dimension,
        }
    }

    pub fn add_document(&mut self, document: Document) -> Result<(), RagError> {
        if document.embedding.len() != self.dimension {
            return Err(RagError::EmbeddingError(
                format!("Expected embedding dimension {}, got {}", 
                       self.dimension, document.embedding.len())
            ));
        }
        self.documents.push(document);
        Ok(())
    }

    pub fn search(&self, query_embedding: &[f32], k: usize) -> Result<Vec<SimilarityResult>, RagError> {
        if query_embedding.len() != self.dimension {
            return Err(RagError::EmbeddingError(
                format!("Query embedding dimension {} doesn't match store dimension {}", 
                       query_embedding.len(), self.dimension)
            ));
        }

        let mut similarities: Vec<SimilarityResult> = self.documents
            .iter()
            .map(|doc| {
                let score = cosine_similarity(query_embedding, &doc.embedding);
                SimilarityResult {
                    document: doc.clone(),
                    score,
                }
            })
            .collect();

        // Sort by similarity score in descending order
        similarities.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Return top k results
        Ok(similarities.into_iter().take(k).collect())
    }

    pub fn len(&self) -> usize {
        self.documents.len()
    }
}

// Gemini API client
pub struct GeminiClient {
    api_key: String,
    base_url: String,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        }
    }

    // Generate embedding for text using Gemini API
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>, RagError> {
        let client = reqwest::Client::new();
        let url = format!("{}/models/gemini-embedding-exp-03-07:embedContent?key={}", 
                         self.base_url, self.api_key);

        let request_body = serde_json::json!({
            "content": {
                "parts": [{"text": text}]
            }
        });

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| RagError::ApiError(e.to_string()))?;
        sleep(time::Duration::from_millis(15_000));

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RagError::ApiError(format!("API request failed: {}", error_text)));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| RagError::ApiError(e.to_string()))?;

        let embedding = response_json["embedding"]["values"]
            .as_array()
            .ok_or_else(|| RagError::ApiError("Invalid embedding response format".to_string()))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }

    // Generate text using Gemini API
    pub async fn generate_text(&self, prompt: &str) -> Result<String, RagError> {
        let client = reqwest::Client::new();
        let url = format!("{}/models/gemini-2.5-flash-lite-preview-06-17:generateContent?key={}", 
                         self.base_url, self.api_key);

        let request_body = serde_json::json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }]
        });

        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| RagError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(RagError::ApiError(format!("API request failed: {}", error_text)));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| RagError::ApiError(e.to_string()))?;

        let text = response_json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| RagError::ApiError("Invalid text generation response format".to_string()))?
            .to_string();

        Ok(text)
    }
}

// Main RAG system
pub struct GeminiRAG {
    client: GeminiClient,
    vector_store: VectorStore,
}

impl GeminiRAG {
    pub fn new(api_key: String, embedding_dimension: usize) -> Self {
        Self {
            client: GeminiClient::new(api_key),
            vector_store: VectorStore::new(embedding_dimension),
        }
    }

    // Add document to the knowledge base
    pub async fn add_document(&mut self, id: String, text: String, metadata: HashMap<String, String>) -> Result<(), RagError> {
        let embedding = self.client.generate_embedding(&text).await?;
        
        let document = Document {
            id,
            text,
            embedding,
            metadata,
        };

        self.vector_store.add_document(document)
    }

    // Retrieve relevant documents for a query
    pub async fn retrieve(&self, query: &str, k: usize) -> Result<Vec<SimilarityResult>, RagError> {
        let query_embedding = self.client.generate_embedding(query).await?;
        self.vector_store.search(&query_embedding, k)
    }

    // Generate answer using RAG approach
    pub async fn generate_answer(&self, query: &str, k: usize) -> Result<String, RagError> {
        // Retrieve relevant documents
        let relevant_docs = self.retrieve(query, k).await?;

        if relevant_docs.is_empty() {
            return Err(RagError::InvalidInput("No relevant documents found".to_string()));
        }

        // Build context from retrieved documents
        let context = relevant_docs
            .iter()
            .map(|result| format!("Document (score: {:.3}): {}", result.score, result.document.text))
            .collect::<Vec<_>>()
            .join("\n\n");

        // Create prompt with context
        let prompt = format!(
            "Based on the following context, please answer the question.\n\nContext:\n{}\n\nQuestion: {}\n\nAnswer:",
            context, query
        );

        // Generate answer using Gemini
        self.client.generate_text(&prompt).await
    }

    pub fn document_count(&self) -> usize {
        self.vector_store.len()
    }
}

// Utility function to calculate cosine similarity
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}


fn jianlai() -> [&'static str; 55]
{
    let sentences: [&str; 55] = [
    "二月二，龍抬頭。",
    "暮色裏，小鎮名叫泥瓶巷的僻靜地方，有位孤苦伶仃的清瘦少年，此時他正按照習俗，一手持蠟燭，一手持桃枝，照耀房梁、牆壁、木床等處，用桃枝敲敲打打，試圖借此驅趕蛇蠍、蜈蚣等，嘴裏念念有詞，是這座小鎮祖祖輩輩傳下來的老話：二月二，燭照梁，桃打牆，人間蛇蟲無處藏。",
    "少年姓陳，名平安，爹娘早逝。",
    "小鎮的瓷器極負盛名，本朝開國以來，就擔當起“奉詔監燒獻陵祭器”的重任，有朝廷官員常年駐紮此地，監理官窯事務。",
    "無依無靠的少年，很早就當起了燒瓷的窯匠，起先只能做些雜事粗活，跟著一個脾氣糟糕的半路師傅，辛苦熬了幾年，剛剛琢磨到一點燒瓷的門道，結果世事無常，小鎮突然失去了官窯造辦這張護身符，小鎮周邊數十座形若臥龍的窯爐，一夜之間全部被官府勒令關閉熄火。",
    "陳平安放下新折的那根桃枝，吹滅蠟燭，走出屋子後，坐在台階上，仰頭望去，星空璀璨。",
    "少年至今仍然清晰記得，那個只肯認自己做半個徒弟的老師傅，姓姚，在去年暮秋時分的清晨，被人發現坐在一張小竹椅子上，正對著窯頭方向，閉眼了。",
    "不過如姚老頭這般鑽牛角尖的人，終究少數。",
    "世世代代都只會燒瓷一事的小鎮匠人，既不敢僭越燒製貢品官窯，也不敢將庫藏瓷器私自販賣給百姓，只得紛紛另謀出路，十四歲的陳平安也被掃地出門，回到泥瓶巷後，繼續守著這棟早已破敗不堪的老宅，差不多是家徒四壁的慘淡場景，便是陳平安想要當敗家子，也無從下手。",
    "當了一段時間飄來蕩去的孤魂野鬼，少年實在找不到掙錢的營生，靠著那點微薄積蓄，少年勉強填飽肚子，前幾天聽說幾條街外的騎龍巷，來了個姓阮的外鄉老鐵匠，對外宣稱要收七八個打鐵的學徒，不給工錢，但管飯，陳平安就趕緊跑去碰運氣，不曾想老人只是斜瞥了他一眼，就把他拒之門外，當時陳平安就納悶，難道打鐵這門活計，不是看臂力大小，而是看面相好壞？",
    "要知道陳平安雖然看著孱弱，但力氣不容小覷，這是少年那些年燒瓷拉坯鍛煉出來的身體底子，除此之外，陳平安還跟著姓姚的老人，跑遍了小鎮方圓百里的山山水水，嚐遍了四周各種土壤的滋味，任勞任怨，什麼髒活累活都願意做，毫不拖泥帶水。",
    "可惜老姚始終不喜歡陳平安，嫌棄少年沒有悟性，是榆木疙瘩不開竅，遠遠不如大徒弟劉羨陽，這也怪不得老人偏心，師父領進門，修行在個人，例如同樣是枯燥乏味的拉坯，劉羨陽短短半年的功力，就抵得上陳平安辛苦三年的水準。",
    "雖然這輩子都未必用得著這門手藝，但陳平安仍是像以往一般，閉上眼睛，想象自己身前擱置有青石板和軲轆車，開始練習拉坯，熟能生巧。",
    "大概每過一刻鍾，少年就會歇息稍許時分，抖抖手腕，如此循環反復，直到整個人徹底精疲力盡，陳平安這才起身，一邊在院中散步，一邊緩緩舒展筋骨。",
    "從來沒有人教過陳平安這些，是他自己瞎琢磨出來的門道。",
    "天地間原本萬籟寂靜，陳平安聽到一聲刺耳的譏諷笑聲，停下腳步，果不其然，看到那個同齡人蹲在牆頭上，咧著嘴，毫不掩飾他的鄙夷神色。",
    "此人是陳平安的老鄰居，據說更是前任監造大人的私生子，那位大人唯恐清流非議、言官彈劾，最後孤身返回京城述職，把孩子交由頗有私交情誼的接任官員，幫著看管照拂。",
    "如今小鎮莫名其妙地失去官窯燒製資格，負責替朝廷監理窯務的督造大人，自己都泥菩薩過江自身難保了，哪裏還顧得上官場同僚的私生子，丟下一些銀錢，就火急火燎趕往京城打點關係。",
    "不知不覺已經淪為棄子的鄰居少年，日子倒是依舊過得優哉游哉，成天帶著他的貼身丫鬟，在小鎮內外逛蕩，一年到頭遊手好閒，也從來不曾為銀子發過愁。",
    "泥瓶巷家家戶戶的黃土院牆都很低矮，其實鄰居少年完全不用踮起腳跟，就可以看到這邊院子的景象，可每次跟陳平安說話，偏偏喜歡蹲在牆頭上。",
    "相比陳平安這個名字的粗淺俗氣，鄰居少年就要雅致許多，叫宋集薪，就連與他相依為命的婢女，也有個文縐縐的稱呼，稚圭。",
    "少女此時就站在院牆那邊，她有一雙杏眼，怯怯弱弱。",
    "院門那邊，有個嗓音響起，“你這婢女賣不賣？”",
    "宋集薪愣了愣，循著聲音轉頭望去，是個眉眼含笑的錦衣少年，站在院外，一張全然陌生的面孔。",
    "錦衣少年身邊站著一位身材高大的老者，面容白皙，臉色和藹，輕輕眯眼打量著兩座毗鄰院落的少年少女。",
    "老者的視線在陳平安一掃而過，並無停滯，但是在宋集薪和婢女身上，多有停留，笑意漸漸濃郁。",
    "宋集薪斜眼道：“賣！怎麼不賣！”",
    "那少年微笑道：“那你說個價。”",
    "少女瞪大眼眸，滿臉匪夷所思，像一頭驚慌失措的年幼麋鹿。",
    "宋集薪翻了個白眼，伸出一根手指，晃了晃，“白銀一萬兩！”",
    "錦衣少年臉色如常，點頭道：“好。”",
    "宋集薪見那少年不像是開玩笑的樣子，連忙改口道：“是黃金萬兩！”",
    "錦衣少年嘴角翹起，道：“逗你玩的。”",
    "宋集薪臉色陰沉。",
    "錦衣少年不再理睬宋集薪，偏移視線，望向陳平安，“今天多虧了你，我才能買到那條鯉魚，買回去後，我越看越歡喜，想著一定要當面跟你道一聲謝，於是就讓吳爺爺帶我連夜來找你。”",
    "他丟出一隻沉甸甸的繡袋，拋給陳平安，笑臉燦爛道：“這是酬謝，你我就算兩清了。”",
    "陳平安剛想要說話，錦衣少年已經轉身離去。",
    "陳平安皺了皺眉頭。",
    "白天自己無意間看到有個中年人，提著隻魚簍走在大街上，捕獲了一尾巴掌長短的金黃鯉魚，它在竹簍裏蹦跳得厲害，陳平安只瞥了一眼，就覺得很喜慶，於是開口詢問，能不能用十文錢買下它，中年人本來只是想著犒勞犒勞自己的五臟廟，眼見有利可圖，就坐地起價，獅子大開口，非要三十文錢才肯賣。",
    "囊中羞澀的陳平安哪裏有這麼多閒錢，又實在舍不得那條金燦燦的鯉魚，就眼饞跟著中年人，軟磨硬泡，想著把價格砍到十五文，哪怕是二十文也行，就在中年人有鬆口跡象的時候，錦衣少年和高大老人正好路過，他們二話不說，用五十文錢買走了鯉魚和魚簍，陳平安只能眼睜睜看著他們揚長而去，無可奈何。",
    "死死盯住那對爺孫愈行愈遠的背影，宋集薪收回惡狠狠的眼神後，跳下牆頭，似乎記起什麼，對陳平安說道：“你還記得正月裏的那條四腳嗎？”",
    "陳平安點了點頭。",
    "怎麼會不記得，簡直就是記憶猶新。",
    "按照這座小鎮傳承數百年的風俗，如果有蛇類往自家屋子鑽，是好兆頭，主人絕對不要將其驅逐打殺。",
    "宋集薪在正月初一的時候，坐在門檻上曬太陽，然後就有隻俗稱四腳蛇的小玩意兒，在他的眼皮子底下往屋裏竄，宋集薪一把抓住就往院子裏摔出去，不曾想那條已經摔得七葷八素的四腳蛇，愈挫愈勇，一次次，把從來不信鬼神之說的宋集薪給氣得不行，一怒之下就把它甩到了陳平安院子，哪裏想到，宋集薪第二天就在自己床底下，看到了那條盤踞蜷縮起來的四腳蛇。",
    "宋集薪察覺到少女扯了扯自己袖子。",
    "少年與她心有靈犀，下意識就將已經到了嘴邊的話語，重新咽回肚子。",
    "他想說的是，那條奇醜無比的四腳蛇，最近額頭上有隆起，如頭頂生角。",
    "宋集薪換了一句話說出口，“我和稚圭可能下個月就要離開這裏了。”",
    "陳平安歎了口氣，“路上小心。”",
    "宋集薪半真半假道：“有些物件我肯定搬不走，你可別趁我家沒人，就肆無忌憚地偷東西。”",
    "陳平安搖了搖頭。",
    "宋集薪驀然哈哈大笑，用手指點了點陳平安，嬉皮笑臉道：“膽小如鼠，難怪寒門無貴子，莫說是這輩子貧賤任人欺，說不定下輩子也逃不掉。”",
    "陳平安默不作聲。",
    "各自返回屋子，陳平安關上門，躺在堅硬的木板床上，貧寒少年閉上眼睛，小聲呢喃道：“碎碎平，歲歲安，碎碎平安，歲歲平安……”",
];
sentences
}


pub struct ElementSystem<'a> {
    pub name: &'a str,
    pub elements: &'a [(&'a str, &'a [&'a str])],
}

pub struct NaturalElementsVocabulary<'a> {
    pub systems: &'a [ElementSystem<'a>],
    pub universal_terms: &'a [&'a str],
}

// 中國五行系統
const WU_XING: &[(&str, &[&str])] = &[
    ("金", &["礦物", "金屬", "鋒利", "收斂", "變革", "肅殺", "西方", "秋天", "肺", "大腸", "白色"]),
    ("木", &["樹木", "植物", "生長", "生發", "條達", "東方", "春天", "肝", "膽", "綠色"]),
    ("水", &["河流", "湖泊", "海洋", "雨", "雪", "流動", "向下", "滋潤", "潛藏", "北方", "冬天", "腎", "膀胱", "黑色"]),
    ("火", &["火焰", "熱", "光明", "向上", "炎上", "溫熱", "南方", "夏天", "心", "小腸", "紅色"]),
    ("土", &["大地", "土壤", "山", "穩定", "承載", "化育", "中央", "季節交替", "脾", "胃", "黃色"]),
];

// 古希臘四元素+以太
const CLASSICAL_ELEMENTS: &[(&str, &[&str])] = &[
    ("地", &["土壤", "岩石", "山脈", "穩定", "堅固", "物質", "身體", "乾燥", "寒冷", "黑色", "棕色"]),
    ("水", &["海洋", "河流", "湖泊", "雨", "液體", "流動", "適應", "情感", "潮濕", "寒冷", "藍色", "綠色"]),
    ("火", &["火焰", "太陽", "熱", "光", "能量", "轉化", "熱情", "意志", "乾燥", "炎熱", "紅色", "橙色"]),
    ("氣", &["大氣", "風", "呼吸", "氣體", "心智", "思想", "溝通", "自由", "流動", "濕潤", "炎熱", "黃色", "白色"]),
    ("以太", &["天空", "虛空", "精微能量", "生命力", "靈性", "宇宙", "第五元素", "超越物質", "紫色", "金色", "透明"]),
];

// 印度五大元素
const PANCHA_BHOOTA: &[(&str, &[&str])] = &[
    ("地", &["固體", "穩定", "支撐", "嗅覺", "骨骼肌肉"]),
    ("水", &["液體", "流動", "凝聚", "味覺", "體液", "血液", "淋巴"]),
    ("火", &["熱", "光", "能量", "轉化", "視覺", "新陳代謝"]),
    ("風", &["氣體", "運動", "觸覺", "呼吸", "神經系統"]),
    ("空", &["空間", "振動", "容納", "聽覺", "心靈", "意識", "連接萬物"]),
];

// 第五元素通用概念
const FIFTH_ELEMENT: &[&str] = &[
    "精神", "生命能量", "意識", "靈魂", "宇宙本質", "連接力", 
    "普拉納", "氣", "瑪那", "奧德", "乙太體", "星光體", 
    "生命力", "意識場", "宇宙意識", "神性"
];

// 跨體系通用術語
const UNIVERSAL_TERMS: &[&str] = &[
    "風", "雨", "雷", "電", "雲", "霧", "霜", "露", "地震", "火山", 
    "潮汐", "季節", "春", "夏", "秋", "冬", "晝夜", "固體", "液體", 
    "氣體", "等離子體", "能量", "光", "熱", "輻射", "東", "南", "西", 
    "北", "中", "早晨", "中午", "下午", "傍晚", "夜晚", "生長", "毀滅", 
    "創造", "維持", "流動", "平衡", "循環", "轉化", "平衡", "和諧", 
    "衝突", "相生", "相剋", "循環", "基礎", "根本", "本質", "構成", 
    "自然哲學", "元素魔法", "鍊金術", "占星術", "風水", "氣功", 
    "阿育吠陀", "傳統中醫學", "神秘學", "形上學"
];

// 整合的詞彙系統
const VOCABULARY_SYSTEMS: &[ElementSystem] = &[
    ElementSystem {
        name: "中國五行",
        elements: WU_XING,
    },
    ElementSystem {
        name: "古希臘元素",
        elements: CLASSICAL_ELEMENTS,
    },
    ElementSystem {
        name: "印度五大元素",
        elements: PANCHA_BHOOTA,
    },
    ElementSystem {
        name: "第五元素概念",
        elements: &[("第五元素", FIFTH_ELEMENT)],
    }
];


// Example usage
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize RAG system
    let api_key = std::env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY environment variable not set");
    
    let mut rag = GeminiRAG::new(api_key, 3072); // Gemini embedding dimension

    // Add documents to knowledge base
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), "doc1".to_string());
    
    rag.add_document(
        "doc1".to_string(),
        "Rust is a systems programming language that runs fast and prevents segfaults.".to_string(),
        metadata.clone()
    ).await?;

    metadata.insert("source".to_string(), "doc2".to_string());
    rag.add_document(
        "doc2".to_string(),
        "Machine learning involves training algorithms on data to make predictions.".to_string(),
        metadata.clone()
    ).await?;

    metadata.insert("source".to_string(), "doc3".to_string());
    rag.add_document(
        "doc3".to_string(),
        "Vector databases store high-dimensional embeddings for similarity search.".to_string(),
        metadata.clone()
    ).await?;

    // let sentences = jianlai();
    // for (i, sentence) in sentences.iter().enumerate() {
    //     println!("句子 {}: {}", i + 1, sentence);

    //     metadata.insert("source".to_string(), format!("doc{}",i + 1 +3));
    //     rag.add_document(
    //         format!("doc{}",i + 1 +3),
    //         sentence.to_string(),
    //         metadata.clone()
    //     ).await?;
    //     sleep(time::Duration::from_millis(15_000));
    // }

    // Add VOCABULARY_SYSTEMS to RAG documents
    let mut doc_counter = 4; // Starting after the initial 3 documents
    
    for system in VOCABULARY_SYSTEMS {
        println!("Adding vocabulary system: {}", system.name);
        
        // Add system overview document
        let mut system_metadata = HashMap::new();
        system_metadata.insert("source".to_string(), "vocabulary_system".to_string());
        system_metadata.insert("system_name".to_string(), system.name.to_string());
        system_metadata.insert("type".to_string(), "system_overview".to_string());
        
        let system_overview = format!("Elemental system: {} contains the following elements: {}", 
            system.name, 
            system.elements.iter().map(|(name, _)| *name).collect::<Vec<_>>().join(", ")
        );
        
        rag.add_document(
            format!("vocab_system_{}", doc_counter),
            system_overview,
            system_metadata
        ).await?;
        doc_counter += 1;
        
        // Add each element and its associated concepts
        for (element_name, concepts) in system.elements {
            let mut element_metadata = HashMap::new();
            element_metadata.insert("source".to_string(), "vocabulary_element".to_string());
            element_metadata.insert("system_name".to_string(), system.name.to_string());
            element_metadata.insert("element_name".to_string(), element_name.to_string());
            element_metadata.insert("type".to_string(), "element_definition".to_string());
            
            let element_text = format!("In the {} system, the element {} is associated with: {}", 
                system.name, 
                element_name, 
                concepts.join(", ")
            );
            
            rag.add_document(
                format!("vocab_element_{}", doc_counter),
                element_text,
                element_metadata
            ).await?;
            doc_counter += 1;
            
            // Add individual concept documents for better granular search
            for concept in *concepts {
                let mut concept_metadata = HashMap::new();
                concept_metadata.insert("source".to_string(), "vocabulary_concept".to_string());
                concept_metadata.insert("system_name".to_string(), system.name.to_string());
                concept_metadata.insert("element_name".to_string(), element_name.to_string());
                concept_metadata.insert("concept".to_string(), concept.to_string());
                concept_metadata.insert("type".to_string(), "concept_association".to_string());
                
                let concept_text = format!("The concept '{}' belongs to the element '{}' in the {} elemental system", 
                    concept, element_name, system.name);
                
                rag.add_document(
                    format!("vocab_concept_{}", doc_counter),
                    concept_text,
                    concept_metadata
                ).await?;
                doc_counter += 1;
            }
        }
    }
    
    // Add universal terms
    println!("Adding universal terms...");
    for (i, term) in UNIVERSAL_TERMS.iter().enumerate() {
        let mut universal_metadata = HashMap::new();
        universal_metadata.insert("source".to_string(), "universal_terms".to_string());
        universal_metadata.insert("term".to_string(), term.to_string());
        universal_metadata.insert("type".to_string(), "universal_concept".to_string());
        
        let universal_text = format!("'{}' is a universal term that appears across multiple elemental and natural philosophy systems", term);
        
        rag.add_document(
            format!("universal_term_{}", doc_counter),
            universal_text,
            universal_metadata
        ).await?;
        doc_counter += 1;
    }

    println!("Added {} documents to knowledge base", rag.document_count());

    // Query the RAG system
    let query = "What is Rust programming language?";
    let answer = rag.generate_answer(query, 2).await?;
    
    println!("Query: {}", query);
    println!("Answer: {}", answer);

    // Retrieve similar documents
    let similar_docs = rag.retrieve("programming language", 2).await?;
    println!("\nSimilar documents:");
    for result in similar_docs {
        println!("Score: {:.3}, Text: {}", result.score, result.document.text);
    }

    // Query the RAG system
    // let query = "你知道泥瓶巷嗎?";
    // let answer = rag.generate_answer(query, 5).await?;
    
    // println!("Query: {}", query);
    // println!("Answer: {}", answer);

    // // Retrieve similar documents
    // let similar_docs = rag.retrieve("泥瓶巷", 5).await?;
    // println!("\nSimilar documents:");
    // for result in similar_docs {
    //     println!("Score: {:.3}, Text: {}", result.score, result.document.text);
    // }
    
    let similar_docs = rag.retrieve("金", 5).await?;
    println!("\nSimilar documents:");
    for result in similar_docs {
        println!("Score: {:.3}, Text: {}", result.score, result.document.text);
    }
    
    let similar_docs = rag.retrieve("木", 5).await?;
    println!("\nSimilar documents:");
    for result in similar_docs {
        println!("Score: {:.3}, Text: {}", result.score, result.document.text);
    }
    
    let similar_docs = rag.retrieve("水", 5).await?;
    println!("\nSimilar documents:");
    for result in similar_docs {
        println!("Score: {:.3}, Text: {}", result.score, result.document.text);
    }
    
    let similar_docs = rag.retrieve("火", 5).await?;
    println!("\nSimilar documents:");
    for result in similar_docs {
        println!("Score: {:.3}, Text: {}", result.score, result.document.text);
    }
    
    let similar_docs = rag.retrieve("土", 5).await?;
    println!("\nSimilar documents:");
    for result in similar_docs {
        println!("Score: {:.3}, Text: {}", result.score, result.document.text);
    }

    Ok(())
}




// Added 58 documents to knowledge base
// Query: What is Rust programming language?
// Answer: Rust is a systems programming language that runs fast and prevents segfaults.

// Similar documents:
// Score: 0.640, Text: Rust is a systems programming language that runs fast and prevents segfaults.
// Score: 0.543, Text: Machine learning involves training algorithms on data to make predictions.
// Query: 你知道泥瓶巷嗎?
// Answer: 是的，我知道泥瓶巷。

// 根據提供的資訊，泥瓶巷是一個小鎮的僻靜地方。那裡的家家戶戶有低矮的黃土院牆。這個小鎮的匠人世世代代都只燒製瓷器，並且曾經有「奉詔監燒獻陵祭器」的重任，朝廷的官員常年在此地監理官窯事務。然而，後來小鎮卻莫名其妙地失去了官窯燒製資格。

// 此外，在泥瓶巷住著一位孤苦伶仃的清瘦少年陳平安。他遵循習俗，在暮色中驅趕蛇蠍、蜈蚣。在小鎮失去官窯燒製資格後，陳平安也被迫離開了，並回到泥瓶巷破敗的老宅。

// Similar documents:
// Score: 0.716, Text: 泥瓶巷家家戶戶的黃土院牆都很低矮，其實鄰居少年完全不用踮起腳跟，就可以看到這邊院子的景象，可每次跟陳平安說話，偏偏喜歡蹲在牆頭上。
// Score: 0.661, Text: 暮色裏，小鎮名叫泥瓶巷的僻靜地方，有位孤苦伶仃的清瘦少年，此時他正按照習俗，一手持蠟燭，一手持桃枝，照耀房梁、牆壁、木床等處，用桃枝敲敲打打，試圖借此驅趕蛇蠍、蜈蚣等，嘴裏念念有詞，是這座小鎮祖祖輩輩傳下來的老話：二月二，燭照梁，桃打牆，人間蛇蟲無處藏。
// Score: 0.641, Text: 世世代代都只會燒瓷一事的小鎮匠人，既不敢僭越燒製貢品官窯，也不敢將庫藏瓷器私自販賣給百姓，只得紛紛另謀出路，十四歲的陳平安也被掃地出門，回到泥瓶巷後，繼續守著這棟早已破敗不堪的老宅，差不多是家徒四壁的慘淡場景，便是陳平安想要當敗家子，也無從下手。
// Score: 0.621, Text: 如今小鎮莫名其妙地失去官窯燒製資格，負責替朝廷監理窯務的督造大人，自己都泥菩薩過江自身難保了，哪裏還顧得上官場同僚的私生子，丟下一些銀錢，就火急火燎趕往京城打點關係。
// Score: 0.596, Text: 錦衣少年臉色如常，點頭道：“好。”





// Query: What is Rust programming language?
// Answer: Rust is a systems programming language that runs fast and prevents segfaults.

// Similar documents:
// Score: 0.640, Text: Rust is a systems programming language that runs fast and prevents segfaults.
// Score: 0.553, Text: The concept '普拉納' belongs to the element '第五元素' in the 第五元素概念 elemental system

// Similar documents:
// Score: 0.694, Text: The concept '白色' belongs to the element '金' in the 中國五行 elemental system
// Score: 0.693, Text: The concept '西方' belongs to the element '金' in the 中國五行 elemental system
// Score: 0.690, Text: The concept '秋天' belongs to the element '金' in the 中國五行 elemental system
// Score: 0.689, Text: The concept '鋒利' belongs to the element '金' in the 中國五行 elemental system
// Score: 0.688, Text: In the 中國五行 system, the element 金 is associated with: 礦物, 金屬, 鋒利, 收斂, 變革, 肅殺, 西方, 秋天, 肺, 大腸, 白色

// Similar documents:
// Score: 0.754, Text: The concept '樹木' belongs to the element '木' in the 中國五行 elemental system
// Score: 0.729, Text: The concept '植物' belongs to the element '木' in the 中國五行 elemental system
// Score: 0.718, Text: The concept '肝' belongs to the element '木' in the 中國五行 elemental system
// Score: 0.717, Text: The concept '綠色' belongs to the element '木' in the 中國五行 elemental system
// Score: 0.717, Text: In the 中國五行 system, the element 木 is associated with: 樹木, 植物, 生長, 生發, 條達, 東方, 春天, 肝, 膽, 綠色

// Similar documents:
// Score: 0.712, Text: The concept '流動' belongs to the element '水' in the 中國五行 elemental system
// Score: 0.710, Text: The concept '滋潤' belongs to the element '水' in the 中國五行 elemental system
// Score: 0.706, Text: The concept '腎' belongs to the element '水' in the 中國五行 elemental system
// Score: 0.706, Text: In the 古希臘元素 system, the element 水 is associated with: 海洋, 河流, 湖泊, 雨, 液體, 流動, 適應, 情感, 潮濕, 寒冷, 藍色, 綠色
// Score: 0.704, Text: The concept '藍色' belongs to the element '水' in the 古希臘元素 elemental system

// Similar documents:
// Score: 0.707, Text: The concept '炎上' belongs to the element '火' in the 中國五行 elemental system
// Score: 0.707, Text: The concept '紅色' belongs to the element '火' in the 中國五行 elemental system
// Score: 0.705, Text: In the 中國五行 system, the element 火 is associated with: 火焰, 熱, 光明, 向上, 炎上, 溫熱, 南方, 夏天, 心, 小腸, 紅色
// Score: 0.702, Text: The concept '熱' belongs to the element '火' in the 中國五行 elemental system
// Score: 0.701, Text: The concept '向上' belongs to the element '火' in the 中國五行 elemental system

// Similar documents:
// Score: 0.746, Text: The concept '土壤' belongs to the element '土' in the 中國五行 elemental system
// Score: 0.740, Text: The concept '大地' belongs to the element '土' in the 中國五行 elemental system
// Score: 0.733, Text: The concept '穩定' belongs to the element '土' in the 中國五行 elemental system
// Score: 0.724, Text: The concept '土壤' belongs to the element '地' in the 古希臘元素 elemental system
// Score: 0.721, Text: The concept '承載' belongs to the element '土' in the 中國五行 elemental system