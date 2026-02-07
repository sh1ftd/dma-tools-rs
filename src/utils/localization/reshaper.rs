use arabic_reshaper::ArabicReshaper;
use unicode_bidi::BidiInfo;

/// Reshapes Arabic text for correct display in LTR-only renderers like egui.
/// This includes both character reshaping (joining) and Bidi reordering.
pub fn reshape_arabic(text: &str) -> String {
    // 1. Reshape the Arabic characters (connect them)
    let reshaper = ArabicReshaper::default();
    let reshaped = reshaper.reshape(text);
    
    // 2. Handle Bidirectional reordering
    let bidi_info = BidiInfo::new(&reshaped, None);
    if bidi_info.paragraphs.is_empty() {
        return reshaped;
    }
    
    let mut visual_text = String::new();
    for para in &bidi_info.paragraphs {
        let (_levels, runs) = bidi_info.visual_runs(para, para.range.clone());
        
        for run in runs {
            let mut run_text = reshaped[run.clone()].to_string();
            
            // The levels vector in unicode-bidi is indexed by character, 
            // but 'run' uses byte indices. We need the character index for the start of the run.
            let char_idx = reshaped[..run.start].chars().count();
            
            if bidi_info.levels[char_idx].is_rtl() {
                run_text = run_text.chars().rev().collect();
            }
            visual_text.push_str(&run_text);
        }
    }
    
    if visual_text.is_empty() {
        reshaped
    } else {
        visual_text
    }
}
