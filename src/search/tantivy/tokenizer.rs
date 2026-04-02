use tantivy::Index;
use tantivy::tokenizer::{
    LowerCaser, RemoveLongFilter, TextAnalyzer, Token, TokenStream, Tokenizer,
};

use super::fields::SEARCH_CODE_TOKENIZER;
use super::identifier::populate_identifier_boundaries;

#[derive(Clone, Default)]
pub(crate) struct CodeTokenizer;

impl Tokenizer for CodeTokenizer {
    type TokenStream<'a> = CodeTokenStream;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        CodeTokenStream::new(text)
    }
}

pub(crate) fn register_search_tokenizer(index: &Index) {
    let analyzer = TextAnalyzer::builder(CodeTokenizer)
        .filter(RemoveLongFilter::limit(80))
        .filter(LowerCaser)
        .build();
    index.tokenizers().register(SEARCH_CODE_TOKENIZER, analyzer);
}

pub(crate) fn collect_search_tokens(index: &Index, text: &str) -> Vec<String> {
    let Some(mut tokenizer) = index.tokenizers().get(SEARCH_CODE_TOKENIZER) else {
        return Vec::new();
    };
    let mut tokens = Vec::new();
    let mut stream = tokenizer.token_stream(text);
    stream.process(&mut |token| tokens.push(token.text.clone()));
    tokens
}

pub(crate) struct CodeTokenStream {
    tokens: Vec<Token>,
    cursor: usize,
    current: Token,
}

impl CodeTokenStream {
    fn new(text: &str) -> Self {
        let mut tokens = Vec::new();
        tokenize_code_text(text, &mut tokens);
        Self {
            tokens,
            cursor: 0,
            current: Token::default(),
        }
    }
}

impl TokenStream for CodeTokenStream {
    fn advance(&mut self) -> bool {
        let Some(token) = self.tokens.get(self.cursor).cloned() else {
            return false;
        };
        self.current = token;
        self.cursor += 1;
        true
    }

    fn token(&self) -> &Token {
        &self.current
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.current
    }
}

fn tokenize_code_text(text: &str, target: &mut Vec<Token>) {
    target.clear();
    let mut fragment_start = None;
    let mut position = 0;

    for (byte_idx, ch) in text.char_indices() {
        if ch.is_alphanumeric() {
            fragment_start.get_or_insert(byte_idx);
            continue;
        }
        if let Some(start) = fragment_start.take() {
            push_fragment_tokens(text, start, byte_idx, &mut position, target);
        }
    }

    if let Some(start) = fragment_start {
        push_fragment_tokens(text, start, text.len(), &mut position, target);
    }
}

fn push_fragment_tokens(
    text: &str,
    start: usize,
    end: usize,
    position: &mut usize,
    target: &mut Vec<Token>,
) {
    let fragment = &text[start..end];
    if fragment.is_empty() {
        return;
    }

    let mut boundaries = Vec::new();
    populate_identifier_boundaries(fragment, &mut boundaries);

    if boundaries.len() > 2 {
        for range in boundaries.windows(2) {
            let token_start = start + range[0];
            let token_end = start + range[1];
            push_token(text, token_start, token_end, *position, target);
            *position += 1;
        }
        return;
    }

    push_token(text, start, end, *position, target);
    *position += 1;
}

fn push_token(
    text: &str,
    offset_from: usize,
    offset_to: usize,
    position: usize,
    target: &mut Vec<Token>,
) {
    if offset_from >= offset_to {
        return;
    }
    target.push(Token {
        offset_from,
        offset_to,
        position,
        text: text[offset_from..offset_to].to_string(),
        position_length: 1,
    });
}
