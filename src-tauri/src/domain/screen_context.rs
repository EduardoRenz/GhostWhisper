//! Contexto de tela por acessibilidade — tipos e poda **puros** (BR-105).
//!
//! Nada de I/O, D-Bus, threads ou relógio aqui: a árvore chega como
//! [`AccessibleNode`] (DTO mínimo definido pelo adaptador) e sai como texto
//! com orçamento de caracteres. O adaptador AT-SPI é quem conversa com o
//! sistema; este módulo só aplica as regras de leitura.

/// Orçamento do texto entregue ao prompt (`<screen_context>`).
pub const CONTEXT_MAX_CHARS: usize = 2_000;

/// Teto do título da janela (pill e atributo do bloco de prompt).
pub const TITLE_MAX_CHARS: usize = 120;

/// Teto da prévia enviada ao front (tooltip da pill).
pub const PREVIEW_MAX_CHARS: usize = 300;

/// Idade máxima do snapshot usado no prompt; mais velho que isso a pergunta
/// sai sem o bloco (a pill continua mostrando o estado atual).
pub const CONTEXT_FRESH_MS: i64 = 30_000;

/// Snapshot da janela ativa, **já podado** pelo domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenContextSnapshot {
    /// Nome curto do app (`google-chrome`, `code`).
    pub app: String,
    /// Título da janela (truncado em [`TITLE_MAX_CHARS`]).
    pub window_title: String,
    /// Texto extraído (limitado a [`CONTEXT_MAX_CHARS`]).
    pub text: String,
    /// Momento em que o snapshot foi capturado (ms, relógio do serviço).
    pub captured_at_ms: i64,
}

/// Papel simplificado de um nó da árvore de acessibilidade.
///
/// O adaptador mapeia os roles do AT-SPI para estes valores; o domain não
/// conhece a enumeração do AT-SPI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    /// Estrutura pura (`application`, `frame`, `panel`, `filler`, `scroll
    /// pane`): navega, mas não contribute texto por si só.
    Structure,
    /// Cabeçalho/label/link/texto estático: contribui `name`.
    Text,
    /// Campo de texto (`entry`, `document`, `paragraph`): contribui `value`.
    Field,
    /// Qualquer outro role: contribui `name` se houver.
    Other,
}

/// Nó da árvore, já normalizado pelo adaptador.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibleNode {
    pub role: NodeRole,
    /// Nome/rotulo acessível (`name` do AT-SPI).
    pub name: String,
    /// Valor/texto do campo (`value`/`text` do AT-SPI).
    pub value: String,
    /// O nó está oculto (estado `invisible`/`hidden`)?
    pub invisible: bool,
    pub children: Vec<AccessibleNode>,
}

impl AccessibleNode {
    /// Construtor compacto para testes e adaptadores.
    #[allow(dead_code)] // usado em testes; adaptadores montam por struct literal
    pub fn new(role: NodeRole, name: &str, value: &str) -> Self {
        Self {
            role,
            name: name.to_string(),
            value: value.to_string(),
            invisible: false,
            children: Vec::new(),
        }
    }

    /// Adiciona um filho (encadeável).
    #[allow(dead_code)] // usado em testes
    pub fn with_child(mut self, child: AccessibleNode) -> Self {
        self.children.push(child);
        self
    }

    /// Marca o nó como oculto (encadeável).
    #[allow(dead_code)] // usado em testes
    pub fn hidden(mut self) -> Self {
        self.invisible = true;
        self
    }
}

/// Poda a árvore em texto, na ordem de leitura, respeitando o orçamento.
///
/// Regras:
/// - nós ocultos são ignorados (inclusive os filhos);
/// - [`NodeRole::Structure`] só navega (contribui o que os filhos contribuem);
/// - [`NodeRole::Text`] contribui `name`; [`NodeRole::Field`] contribui
///   `value` (com fallback para `name`);
/// - [`NodeRole::Other`] contribui `name` quando não vazio;
/// - linhas idênticas consecutivas são deduplicadas;
/// - o corte final respeita `budget` e termina com `…`.
pub fn extract_text(root: &AccessibleNode, budget: usize) -> String {
    let mut lines = Vec::new();
    collect_lines(root, &mut lines);
    let joined = join_lines(&lines);
    truncate_budget(&joined, budget)
}

/// Trunca o título da janela em [`TITLE_MAX_CHARS`].
pub fn truncate_title(title: &str) -> String {
    truncate_budget(title, TITLE_MAX_CHARS)
}

/// Corta `text` em `budget` chars sem cortar a string no meio de um char
/// (byte boundary) e anexando `…` quando há corte.
fn truncate_budget(text: &str, budget: usize) -> String {
    if text.chars().count() <= budget {
        return text.to_string();
    }
    let cut: String = text.chars().take(budget.saturating_sub(1)).collect();
    format!("{cut}…")
}

/// Coleta as linhas de texto da árvore em ordem de leitura (pré-ordem).
fn collect_lines(node: &AccessibleNode, lines: &mut Vec<String>) {
    if node.invisible {
        return;
    }
    if let Some(line) = node_line(node) {
        let line = line.trim().to_string();
        if !line.is_empty() {
            lines.push(line);
        }
    }
    for child in &node.children {
        collect_lines(child, lines);
    }
}

/// Texto contribuído pelo próprio nó (sem os filhos).
///
/// **Pública** para o adaptador AT-SPI: a caminhada usa o mesmo critério do
/// domain para a **parada antecipada** (acumula chars e corta ao chegar no
/// orçamento — ISS-092), sem divergir da poda final.
pub fn node_line(node: &AccessibleNode) -> Option<String> {
    match node.role {
        NodeRole::Structure => None,
        NodeRole::Text => non_empty(&node.name),
        NodeRole::Field => non_empty(&node.value).or_else(|| non_empty(&node.name)),
        NodeRole::Other => non_empty(&node.name),
    }
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Junta as linhas com `\n`, removendo **linhas idênticas consecutivas**.
fn join_lines(lines: &[String]) -> String {
    let mut out = String::new();
    let mut last: Option<&str> = None;
    for line in lines {
        if last == Some(line.as_str()) {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line);
        last = Some(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(name: &str) -> AccessibleNode {
        AccessibleNode::new(NodeRole::Text, name, "")
    }

    fn field(value: &str) -> AccessibleNode {
        AccessibleNode::new(NodeRole::Field, "", value)
    }

    fn structure() -> AccessibleNode {
        AccessibleNode::new(NodeRole::Structure, "", "")
    }

    #[test]
    fn extracts_text_in_reading_order() {
        // Pré-ordem: o nome do pai antes dos filhos.
        let root = structure()
            .with_child(text("Título da página"))
            .with_child(text("Parágrafo um"))
            .with_child(text("Parágrafo dois"));

        assert_eq!(
            extract_text(&root, CONTEXT_MAX_CHARS),
            "Título da página\nParágrafo um\nParágrafo dois"
        );
    }

    #[test]
    fn skips_hidden_nodes_and_their_children() {
        let root = structure()
            .with_child(text("visível"))
            .with_child(text("oculto").hidden())
            .with_child(structure().hidden().with_child(text("filho de oculto")));

        assert_eq!(extract_text(&root, CONTEXT_MAX_CHARS), "visível");
    }

    #[test]
    fn uses_value_for_fields_and_name_for_text() {
        let root = structure()
            .with_child(text("Busca"))
            .with_child(field("whisper local"));

        assert_eq!(
            extract_text(&root, CONTEXT_MAX_CHARS),
            "Busca\nwhisper local"
        );
    }

    #[test]
    fn field_falls_back_to_name_when_value_empty() {
        let node = AccessibleNode::new(NodeRole::Field, "fallback", "");
        assert_eq!(extract_text(&node, CONTEXT_MAX_CHARS), "fallback");
    }

    #[test]
    fn dedupes_consecutive_identical_lines() {
        let root = structure()
            .with_child(text("menu"))
            .with_child(text("menu"))
            .with_child(text("outro"))
            .with_child(text("menu"));

        assert_eq!(extract_text(&root, CONTEXT_MAX_CHARS), "menu\noutro\nmenu");
    }

    #[test]
    fn structure_nodes_contribute_no_text() {
        let root = structure().with_child(structure()).with_child(text("ok"));
        assert_eq!(extract_text(&root, CONTEXT_MAX_CHARS), "ok");
    }

    #[test]
    fn respects_char_budget() {
        let long = "x".repeat(CONTEXT_MAX_CHARS + 500);
        let node = text(&long);
        let out = extract_text(&node, CONTEXT_MAX_CHARS);

        assert_eq!(out.chars().count(), CONTEXT_MAX_CHARS);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn empty_tree_yields_empty_text() {
        assert_eq!(extract_text(&structure(), CONTEXT_MAX_CHARS), "");
        assert_eq!(extract_text(&text("   "), CONTEXT_MAX_CHARS), "");
    }

    #[test]
    fn truncate_title_respects_limit() {
        let title = "t".repeat(TITLE_MAX_CHARS + 40);
        let out = truncate_title(&title);
        assert_eq!(out.chars().count(), TITLE_MAX_CHARS);
        assert!(out.ends_with('…'));

        assert_eq!(truncate_title("curto"), "curto");
    }

    #[test]
    fn budget_cut_lands_on_char_boundary() {
        // Texto multibyte: cortar em chars, nunca em bytes quebrados.
        let node = text(&"é".repeat(100));
        let out = extract_text(&node, 10);
        assert_eq!(out.chars().count(), 10);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn node_line_is_the_shared_walk_criterion() {
        // ISS-092: `node_line` é o critério público que o adaptador usa para
        // a parada antecipada — tem que bater com o que `extract_text`
        // efetivamente coletaria de cada role (senão a caminhada corta cedo
        // ou tarde demais).
        assert_eq!(node_line(&structure()), None, "estrutura não conta");
        assert_eq!(node_line(&text("título")).as_deref(), Some("título"));
        assert_eq!(node_line(&field("valor")).as_deref(), Some("valor"));
        let other = AccessibleNode::new(NodeRole::Other, "botão", "");
        assert_eq!(node_line(&other).as_deref(), Some("botão"));
        let hidden_text = text("oculto").hidden();
        // A poda final pula ocultos; `node_line` não decide isso (o adaptador
        // já marca `invisible` antes) — contrato: contar só o conteúdo.
        assert_eq!(node_line(&hidden_text).as_deref(), Some("oculto"));
    }
}
