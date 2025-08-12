use dotenv::dotenv;
use std::env;
use frankenstein::client_reqwest::Bot;
use frankenstein::methods::{GetUpdatesParams, SendMessageParams, GetFileParams, SetMyCommandsParams, SendPhotoParams};
use frankenstein::updates::UpdateContent;
use frankenstein::types::{Message, ChatType, BotCommand};
use frankenstein::AsyncTelegramApi;
use frankenstein::input_file::{FileUpload, InputFile};
use tokio::time::{sleep, Duration};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::collections::HashMap;
use std::path::Path;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;
use regex::Regex;

// Estrutura para um inscrito no evento
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Inscrito {
    id: u32,
    nome: String,
    user: String,
}

// Estrutura para uma missão
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Missao {
    titulo: String,
    texto: String,
}

// Estrutura para uma entrega de missão
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Entrega {
    nome: String,
    user: String,
    time: String,
    imagens: Vec<String>,
    textos: Vec<String>,
}

// Estrutura para um naipe de missões
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct Naipe {
    pedra: u32,
    papel: u32,
    tesoura: u32,
}

// Estrutura para o banco de dados de um time
#[derive(Serialize, Deserialize, Debug, Clone)]
struct TimeDB {
    soldados: i32,
    naipes: Vec<Naipe>,
}

// Estrutura para manter o estado da conversa
#[derive(Serialize, Deserialize, Debug, Clone)]
struct UserState {
    step: String,
    time: Option<String>,
    entregas: Vec<String>,
}

// Mapa para rastrear o estado de cada usuário, protegido por Arc<Mutex<>>
type UserStates = Arc<Mutex<HashMap<i64, UserState>>>;

// New struct to hold parsed mission emojis
#[derive(Debug, Clone, Default)]
struct MissionEmojis {
    emojis: HashMap<usize, HashMap<String, String>>, // Naipe index -> (Type -> Emoji)
}

fn parse_missoes_emojis(missoes_text: &str) -> MissionEmojis {
    let mut mission_emojis = MissionEmojis { emojis: HashMap::new() };
    let naipe_sections: Vec<&str> = missoes_text.split("Naipe ").collect();

    for section in naipe_sections.iter().skip(1) { // Skip the first part before "Naipe 01"
        if let Some(first_line_end) = section.find('\n') {
            let first_line = &section[..first_line_end];
            let rest_of_section = &section[first_line_end..];

            let naipe_num_re = Regex::new(r"^(\d{1,2})").unwrap(); // Changed to \d{1,2} to match 1 or 2 digits
            if let Some(caps) = naipe_num_re.captures(first_line) {
                let naipe_index = caps[1].parse::<usize>().unwrap();
                let mission_type_re = Regex::new(r"● ([^\s]+) .+? \((Pedra|Papel|Tesoura)\)").unwrap();
                let mut current_naipe_emojis = HashMap::new();

                for mission_caps in mission_type_re.captures_iter(rest_of_section) {
                    let emoji = mission_caps[1].trim().to_string();
                    let mission_type = mission_caps[2].to_string();
                    current_naipe_emojis.insert(mission_type, emoji);
                }
                mission_emojis.emojis.insert(naipe_index, current_naipe_emojis);
            }
        }
    }
    mission_emojis
}

fn read_inscritos() -> Result<Vec<Inscrito>, String> {
    if !Path::new("inscritos.json").exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string("inscritos.json").map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn write_inscritos(inscritos: &Vec<Inscrito>) -> Result<(), String> {
    let data = serde_json::to_string_pretty(inscritos).map_err(|e| e.to_string())?;
    fs::write("inscritos.json", data).map_err(|e| e.to_string())
}

fn read_missoes() -> Result<Vec<Missao>, String> {
    if !Path::new("missoes.json").exists() {
        fs::write("missoes.json", "[]").map_err(|e| e.to_string())?;
    }
    let data = fs::read_to_string("missoes.json").map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn read_entregas(time: &str) -> Result<Vec<Entrega>, String> {
    let file_path = format!("registro_{}.json", time);
    if !Path::new(&file_path).exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(file_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn write_entregas(time: &str, entregas: &Vec<Entrega>) -> Result<(), String> {
    let file_path = format!("registro_{}.json", time);
    let data = serde_json::to_string_pretty(entregas).map_err(|e| e.to_string())?;
    fs::write(file_path, data).map_err(|e| e.to_string())
}

fn read_time_db(time: &str) -> Result<TimeDB, String> {
    let file_path = format!("{}.json", time);
    let data = fs::read_to_string(file_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn write_time_db(time: &str, db: &TimeDB) -> Result<(), String> {
    let file_path = format!("{}.json", time);
    let data = serde_json::to_string_pretty(db).map_err(|e| e.to_string())?;
    fs::write(file_path, data).map_err(|e| e.to_string())
}

fn inicializar_times() {
    let times = ["shu", "wei", "wu"];
    for time in &times {
        let file_path = format!("{}.json", time);
        if !Path::new(&file_path).exists() {
            let naipes = vec![Naipe::default(); 22];
            let db = TimeDB {
                soldados: 10000,
                naipes,
            };
            let data = serde_json::to_string_pretty(&db).unwrap();
            if let Err(e) = fs::write(&file_path, data) {
                println!("Falha ao criar o banco de dados para o time {}: {}", time, e);
            } else {
                println!("Banco de dados para o time {} criado com sucesso.", time);
            }
        }
    }
}

fn get_team_group_id(team_name: &str) -> i64 {
    let group_id_str = match team_name {
        "shu" => env::var("SHU_GROUP_ID").unwrap_or_else(|_| "0".to_string()),
        "wei" => env::var("WEI_GROUP_ID").unwrap_or_else(|_| "0".to_string()),
        "wu" => env::var("WU_GROUP_ID").unwrap_or_else(|_| "0".to_string()),
        _ => "0".to_string(),
    };
    group_id_str.parse().unwrap_or(0)
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let bot = Arc::new(Bot::new(&token));

    if let Err(e) = set_menu_commands(bot.clone()).await {
        println!("Falha ao configurar o menu de comandos: {}", e);
    }

    inicializar_times();

    let mut update_params = GetUpdatesParams::builder().build();
    let user_states: UserStates = Arc::new(Mutex::new(HashMap::new()));

    println!("Yuan Shao Bot está de prontidão!");

    loop {
        let result = bot.get_updates(&update_params).await;
        match result {
            Ok(response) => {
                for update in response.result {
                    if let UpdateContent::Message(message) = update.content {
                        let bot_clone = Arc::clone(&bot);
                        let states_clone = Arc::clone(&user_states);
                        tokio::spawn(async move {
                            process_message(*message, bot_clone, states_clone).await;
                        });
                    }
                    update_params.offset = Some((update.update_id + 1) as i64);
                }
            }
            Err(error) => {
                println!("Falha ao buscar atualizações: {:?}", error);
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn process_message(message: Message, bot: Arc<Bot>, user_states: UserStates) {
    println!(
        "Nova mensagem recebida no chat '{}' (ID: {})",
        message.chat.title.as_deref().unwrap_or("Chat Privado"),
        message.chat.id
    );

    let user_id = message.from.as_ref().map_or(0, |u| u.id as i64);

    let state_exists = {
        let states = user_states.lock().await;
        states.contains_key(&user_id)
    };

    if state_exists {
        handle_state_logic(&message, bot, user_states).await;
    } else if let Some(text) = &message.text {
        handle_command_logic(text, &message, bot, user_states).await;
    }
}

async fn handle_state_logic(message: &Message, bot: Arc<Bot>, user_states: UserStates) {
    let chat_id = message.chat.id;
    let user_id = message.from.as_ref().map_or(0, |u| u.id as i64);
    let mut states = user_states.lock().await;
    let state = states.get(&user_id).cloned();

    if let Some(mut current_state) = state {
        match current_state.step.as_str() {
            "aguardando_confirmacao_inscricao" => {
                let user = message.from.as_ref().unwrap();
                let text = message.text.as_deref().unwrap_or("").to_lowercase();
                if ["sim", "s", "ss"].contains(&text.as_str()) {
                    let mut inscritos = read_inscritos().unwrap_or_default();
                    let new_id = inscritos.len() as u32 + 1;
                    let new_inscrito = Inscrito {
                        id: new_id,
                        nome: user.first_name.clone(),
                        user: user.username.as_deref().unwrap_or("").to_string(),
                    };
                    inscritos.push(new_inscrito);
                    if write_inscritos(&inscritos).is_ok() {
                        send_message(chat_id, "Sua lealdade foi registrada! Você agora é um de meus nobres seguidores. Juntos, alcançaremos a glória!", &bot).await;
                    } else {
                        send_message(chat_id, "Houve um erro em meus registros. Tente novamente mais tarde, nobre guerreiro.", &bot).await;
                    }
                } else {
                    send_message(chat_id, "Sua hesitação é compreensível, mas a glória não espera por ninguém. Quando estiver pronto para se juntar a mim, estarei aqui.", &bot).await;
                }
                states.remove(&user_id);
            }
            "aguardando_time" => {
                let time = message.text.as_deref().unwrap_or("").to_lowercase();
                if ["shu", "wei", "wu"].contains(&time.as_str()) {
                    current_state.step = "aguardando_entregas".to_string();
                    current_state.time = Some(time);
                    states.insert(user_id, current_state);
                    send_message(chat_id, "Excelente. Agora, apresente-me as provas de seus feitos. Envie-me suas imagens e textos. Quando terminar, use o comando /entregar para que eu possa avaliar sua bravura.", &bot).await;
                } else {
                    send_message(chat_id, "Guerreiro, essa casa não figura entre as grandes. Escolha entre Shu, Wei ou Wu para que eu possa registrar seus feitos corretamente.", &bot).await;
                }
            }
            "aguardando_entregas" => {
                if let Some(text) = &message.text {
                    if text == "/entregar" {
                        let user = message.from.as_ref().unwrap();
                        let time = current_state.time.as_ref().unwrap();
                        let mut entregas = read_entregas(time).unwrap_or_default();
                        let new_entrega = Entrega {
                            nome: user.first_name.clone(),
                            user: user.username.as_deref().unwrap_or("").to_string(),
                            time: time.clone(),
                            imagens: current_state.entregas.iter().filter(|e| e.starts_with("entregas/")).cloned().collect(),
                            textos: current_state.entregas.iter().filter(|e| !e.starts_with("entregas/")).cloned().collect(),
                        };
                        entregas.push(new_entrega);

                        if write_entregas(time, &entregas).is_ok() {
                            send_delivery_to_admin(&current_state, user, &bot).await;
                            send_message(chat_id, "Seus feitos foram registrados e enviados para avaliação. Sua bravura será reconhecida, nobre guerreiro!", &bot).await;
                        } else {
                            send_message(chat_id, "Houve uma falha em meus arquivos. Peço que tente novamente mais tarde.", &bot).await;
                        }
                        states.remove(&user_id);
                    } else {
                        current_state.entregas.push(text.clone());
                        states.insert(user_id, current_state);
                        send_message(chat_id, "Registrado. Envie mais provas ou use /entregar para finalizar.", &bot).await;
                    }
                } else if let Some(photo) = message.photo.as_ref() {
                    let file_id = &photo.last().unwrap().file_id;
                    let time_str = current_state.time.as_ref().unwrap();
                    match download_file(&bot, file_id, time_str, user_id).await {
                        Ok(path) => {
                            current_state.entregas.push(path);
                            states.insert(user_id, current_state);
                            send_message(chat_id, "Sua imagem foi recebida. Envie mais ou use /entregar.", &bot).await;
                        }
                        Err(e) => {
                            println!("Falha ao baixar imagem: {}", e);
                            send_message(chat_id, "Houve uma falha ao receber sua imagem. Por favor, tente novamente.", &bot).await;
                        }
                    };
                }
            }
            _ => {}
        }
    }
}

async fn handle_command_logic(text: &str, message: &Message, bot: Arc<Bot>, user_states: UserStates) {
    let chat_id = message.chat.id;
    let user_id = message.from.as_ref().map_or(0, |u| u.id as i64);
    let mut states = user_states.lock().await;

    match text {
        "/start" => send_message(chat_id, "Saudações, nobre guerreiro! Eu, Yuan Shao, líder da aliança contra a tirania, dou-lhe as boas-vindas. O que o traz à minha presença?", &bot).await,
        "/inscricao" => {
            if message.chat.type_field != ChatType::Private {
                send_message(chat_id, "Meu nobre, para se juntar à minha causa, peço que me chame em particular. A discrição é uma virtude dos grandes líderes.", &bot).await;
                return;
            }
            let user = message.from.as_ref().unwrap();
            let inscritos = read_inscritos().unwrap_or_default();
            if inscritos.iter().any(|i| i.user == user.username.as_deref().unwrap_or("")) {
                send_message(chat_id, "Guerreiro, sua lealdade já foi registrada. Você já faz parte de minha nobre aliança!", &bot).await;
                return;
            }
            states.insert(user_id, UserState {
                step: "aguardando_confirmacao_inscricao".to_string(),
                time: None,
                entregas: Vec::new(),
            });
            send_message(chat_id, "Você, nobre guerreiro, deseja jurar lealdade a mim, Yuan Shao, e se inscrever em minha gloriosa campanha? Responda com 'sim' para selar seu destino.", &bot).await;
        }
        "/inscritos" => {
            let admin_group_id = env::var("ADMIN_GROUP_ID").expect("ADMIN_GROUP_ID not set");
            if chat_id.to_string() != admin_group_id {
                send_message(chat_id, "Este comando só pode ser utilizado no grupo de administradores.", &bot).await;
                return;
            }
            let inscritos = read_inscritos().unwrap_or_default();
            if inscritos.is_empty() {
                send_message(chat_id, "Minha nobre aliança ainda não possui membros. Seja o primeiro a se juntar à minha causa gloriosa usando /inscricao !", &bot).await;
            } else {
                let mut response = String::from("Estes são os nobres guerreiros que juraram lealdade a mim:\n\n");
                for inscrito in inscritos {
                    response.push_str(&format!("- Inscrição Nº {}: {} (@{})\n", inscrito.id, inscrito.nome, inscrito.user));
                }
                send_message(chat_id, &response, &bot).await;
            }
        }
        "/entregarmissao" => {
            if message.chat.type_field != ChatType::Private {
                send_message(chat_id, "Meu nobre, para me apresentar seus feitos, peço que o faça em particular. A glória de seus atos não deve ser ofuscada.", &bot).await;
                return;
            }
            states.insert(user_id, UserState {
                step: "aguardando_time".to_string(),
                time: None,
                entregas: Vec::new(),
            });
            send_message(chat_id, "Nobre guerreiro, antes de me apresentar seus feitos, diga-me a qual das grandes casas você jurou lealdade? (Shu, Wei ou Wu)", &bot).await;
        }
        "/missoes" => {
            let missoes_data = read_missoes().unwrap_or_default();
            if missoes_data.is_empty() {
                send_message(chat_id, "Não há decretos no momento. Aguardem minhas ordens, a glória nos espera!", &bot).await;
            } else {
                let full_text = missoes_data.get(0).map(|m| m.texto.as_str()).unwrap_or("");
                let lines: Vec<&str> = full_text.lines().collect();

                let mut part1 = String::from("Escutem todos o meu decreto! (Parte 1/2)\n\n");
                let mut part2 = String::from("Escutem todos o meu decreto! (Parte 2/2)\n\n");

                let mut current_part = 1;
                for line in lines {
                    if line.contains("Naipe 11") {
                        current_part = 2;
                    }
                    if current_part == 1 {
                        part1.push_str(line);
                        part1.push('\n');
                    } else {
                        part2.push_str(line);
                        part2.push('\n');
                    }
                }
                
                send_message(chat_id, &part1, &bot).await;
                tokio::time::sleep(Duration::from_secs(1)).await; // Small delay to avoid rate limits
                send_message(chat_id, &part2, &bot).await;
            }
        }
        "/shu" | "/wei" | "/wu" => {
            let admin_group_id = env::var("ADMIN_GROUP_ID").expect("ADMIN_GROUP_ID not set");
            let shu_group_id = env::var("SHU_GROUP_ID").unwrap_or_else(|_| "0".to_string()); // Placeholder
            let wei_group_id = env::var("WEI_GROUP_ID").unwrap_or_else(|_| "0".to_string()); // Placeholder
            let wu_group_id = env::var("WU_GROUP_ID").unwrap_or_else(|_| "0".to_string()); // Placeholder

            let team_name = text.trim_start_matches('/').to_lowercase();
            let chat_id_str = chat_id.to_string();

            if chat_id_str == admin_group_id
                || (team_name == "shu" && chat_id_str == shu_group_id)
                || (team_name == "wei" && chat_id_str == wei_group_id)
                || (team_name == "wu" && chat_id_str == wu_group_id)
            {
                send_team_db(chat_id, &team_name, &bot).await;
            } else {
                send_message(chat_id, "Este comando só pode ser utilizado no grupo de administradores ou no grupo do seu time.", &bot).await;
            }
        }
        _ => {
            // Comandos de Admin
            let admin_group_id = env::var("ADMIN_GROUP_ID").expect("ADMIN_GROUP_ID not set");
            if chat_id.to_string() == admin_group_id {
                if text.starts_with("/add") || text.starts_with("/remove") {
                    handle_admin_commands(text, chat_id, &bot).await;
                }
            }
        }
    }
}

async fn send_message(chat_id: i64, text: &str, bot: &Bot) {
    let params = SendMessageParams::builder()
        .chat_id(chat_id)
        .text(text)
        .build();
    if let Err(err) = bot.send_message(&params).await {
        println!("Falha ao enviar mensagem: {:?}", err);
    }
}

async fn send_delivery_to_admin(state: &UserState, user: &frankenstein::types::User, bot: &Bot) {
    let admin_group_id_str = env::var("ADMIN_GROUP_ID").expect("ADMIN_GROUP_ID not set");
    let admin_group_id = admin_group_id_str.parse().unwrap();
    let time = state.time.as_ref().unwrap();

    let textos: Vec<String> = state.entregas.iter()
        .filter(|e| !e.starts_with("entregas/"))
        .cloned()
        .collect();

    let mut admin_message = format!(
        "Nova entrega de {} (@{}) para o time {}:\n\n",
        user.first_name,
        user.username.as_deref().unwrap_or(""),
        time.to_uppercase()
    );

    if !textos.is_empty() {
        admin_message.push_str("Textos:\n");
        for texto in textos {
            admin_message.push_str(&format!("- {}\n", texto));
        }
    }

    send_message(admin_group_id, &admin_message, bot).await;

    for entrega in &state.entregas {
        if entrega.starts_with("entregas/") {
            let photo_params = SendPhotoParams::builder()
                .chat_id(admin_group_id)
                .photo(FileUpload::InputFile(InputFile { path: entrega.into() }))
                .build();
            if let Err(e) = bot.send_photo(&photo_params).await {
                println!("Falha ao enviar foto para o admin: {}", e);
            }
        }
    }
}

async fn download_file(bot: &Bot, file_id: &str, time: &str, user_id: i64) -> Result<String, String> {
    let get_file_params = GetFileParams::builder().file_id(file_id).build();
    let file = bot.get_file(&get_file_params).await.map_err(|e| e.to_string())?.result;
    let file_path = file.file_path.ok_or("File path not available")?;

    let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let url = format!("https://api.telegram.org/file/bot{}/{}", token, file_path);

    let response = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;

    let dir_path = format!("entregas/{}", time);
    fs::create_dir_all(&dir_path).map_err(|e| e.to_string())?;

    let timestamp = Utc::now().timestamp_millis();
    let file_extension = Path::new(&file_path).extension().and_then(|s| s.to_str()).unwrap_or("jpg");
    let new_file_path = format!("{}/{}_{}.{}", dir_path, user_id, timestamp, file_extension);

    let mut dest = fs::File::create(&new_file_path).map_err(|e| e.to_string())?;
    dest.write_all(&bytes).map_err(|e| e.to_string())?;

    Ok(new_file_path)
}

async fn set_menu_commands(bot: Arc<Bot>) -> Result<(), String> {
    let commands = vec![
        BotCommand::builder()
            .command("/inscricao")
            .description("Jure lealdade e junte-se à minha nobre causa.")
            .build(),
        BotCommand::builder()
            .command("/missoes")
            .description("Consulte meus decretos e missões atuais.")
            .build(),
        BotCommand::builder()
            .command("/entregarmissao")
            .description("Apresente seus feitos para minha avaliação.")
            .build(),
    ];

    let params = SetMyCommandsParams::builder().commands(commands).build();
    bot.set_my_commands(&params).await.map_err(|e| e.to_string())?;
    println!("Menu de comandos configurado com sucesso.");
    Ok(())
}

async fn handle_admin_commands(text: &str, chat_id: i64, bot: &Bot) {
    // Regex para /addsoldados e /removesoldados
    let soldados_re = Regex::new(r"^/(add|remove)soldados\s+(shu|wei|wu)\s+(-?\d+)$").unwrap();
    // Regex para /add e /remove de naipes
    let naipe_re = Regex::new(r"^/(add|remove)\s+(shu|wei|wu)\s+(\d{1,2})\s+(pedra|papel|tesoura)$").unwrap();

    if let Some(caps) = soldados_re.captures(text) {
        let action = &caps[1];
        let time = &caps[2];
        let quantidade = caps[3].parse::<i32>().unwrap();

        let mut db = match read_time_db(time) {
            Ok(db) => db,
            Err(e) => {
                send_message(chat_id, &format!("Falha ao ler DB do time {}: {}", time, e), bot).await;
                return;
            }
        };

        if action == "add" {
            db.soldados += quantidade;
        } else {
            db.soldados -= quantidade;
        }

        if write_time_db(time, &db).is_ok() {
            send_message(chat_id, &format!("Soldados do time {} atualizados. Total: {}", time.to_uppercase(), db.soldados), bot).await;
            // Send notification to team group
            let team_group_id = get_team_group_id(time);
            if team_group_id != 0 { // Check if a valid ID is set
                send_message(team_group_id, &format!("Atenção, nobres guerreiros de {}! Seus soldados foram atualizados. Contamos agora com {} bravos combatentes em nossas fileiras!", time.to_uppercase(), db.soldados), bot).await;
            }
        } else {
            send_message(chat_id, &format!("Falha ao salvar DB do time {}", time), bot).await;
        }
        return;
    }

    if let Some(caps) = naipe_re.captures(text) {
        let action = &caps[1];
        let time = &caps[2];
        let naipe_idx = caps[3].parse::<usize>().unwrap();
        let missao = &caps[4];

        if !(1..=22).contains(&naipe_idx) {
            send_message(chat_id, "Naipe inválido. Deve ser entre 1 e 22.", bot).await;
            return;
        }

        let mut db = match read_time_db(time) {
            Ok(db) => db,
            Err(e) => {
                send_message(chat_id, &format!("Falha ao ler DB do time {}: {}", time, e), bot).await;
                return;
            }
        };

        let delta = if action == "add" { 1 } else { -1 };
        let naipe = &mut db.naipes[naipe_idx - 1];

        match missao {
            "pedra" => naipe.pedra = (naipe.pedra as i32 + delta).max(0) as u32,
            "papel" => naipe.papel = (naipe.papel as i32 + delta).max(0) as u32,
            "tesoura" => naipe.tesoura = (naipe.tesoura as i32 + delta).max(0) as u32,
            _ => {}
        }

        if write_time_db(time, &db).is_ok() {
            send_message(chat_id, &format!("Missão {} do naipe {} para o time {} atualizada.", missao, naipe_idx, time.to_uppercase()), bot).await;
            // Send notification to team group
            let team_group_id = get_team_group_id(time);
            if team_group_id != 0 { // Check if a valid ID is set
                send_message(team_group_id, &format!("Atenção, guerreiros de {}! A missão do naipe {} ({}) foi atualizada em seus registros. Que a glória os acompanhe!", time.to_uppercase(), naipe_idx, missao.to_uppercase()), bot).await;
            }
        } else {
            send_message(chat_id, &format!("Falha ao salvar DB do time {}", time), bot).await;
        }
        return;
    }

    // Se nenhum regex corresponder
    send_message(chat_id, "Comando de admin não reconhecido ou formato inválido.", bot).await;
}

async fn send_team_db(chat_id: i64, team_name: &str, bot: &Bot) {
    let missoes_data = match read_missoes() {
        Ok(m) => m,
        Err(_) => {
            send_message(chat_id, "Falha ao ler dados das missões.", bot).await;
            return;
        }
    };

    let missoes_text = missoes_data.get(0).map(|m| m.texto.as_str()).unwrap_or("");
    let mission_emojis = parse_missoes_emojis(missoes_text);

    let db = match read_time_db(team_name) {
        Ok(db) => db,
        Err(e) => {
            send_message(chat_id, &format!("Falha ao ler o banco de dados do time {}: {}", team_name.to_uppercase(), e), bot).await;
            return;
        }
    };

    let mut response = format!("📊 **Banco de Dados do Time {}** 📊\n\n", team_name.to_uppercase());
    response.push_str(&format!("Soldados: {}\n\n", db.soldados));
    response.push_str("Missões por Naipe:\n");

    for (i, naipe) in db.naipes.iter().enumerate() {
        let naipe_index = i + 1;
        let emojis_for_naipe = mission_emojis.emojis.get(&naipe_index);

        let pedra_emoji = emojis_for_naipe.and_then(|e| e.get("Pedra")).map_or("🛡", |s| s.as_str());
        let papel_emoji = emojis_for_naipe.and_then(|e| e.get("Papel")).map_or("📜", |s| s.as_str());
        let tesoura_emoji = emojis_for_naipe.and_then(|e| e.get("Tesoura")).map_or("✂️", |s| s.as_str());

        response.push_str(&format!(
            "Naipe {}: {} Pedra: {} | {} Papel: {} | {} Tesoura: {}\n",
            naipe_index,
            pedra_emoji, naipe.pedra,
            papel_emoji, naipe.papel,
            tesoura_emoji, naipe.tesoura
        ));
    }

    send_message(chat_id, &response, bot).await;
}
