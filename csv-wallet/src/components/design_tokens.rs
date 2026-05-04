//! Design System Tokens for CSV Wallet.
//!
//! Defines CSS custom properties for colors, spacing, typography, and seal states.
//! These tokens ensure consistency across the UI and enable theming.

use dioxus::prelude::*;

/// Inject design token CSS variables into the document.
pub fn inject_design_tokens() -> Element {
    rsx! {
        style { {CSS_TOKENS} }
    }
}

/// Global CSS custom properties.
const CSS_TOKENS: &str = r#"
:root {
    /* ========================================
       COLOR PALETTE
       ======================================== */
    
    /* Primary brand colors */
    --color-primary-50: #eff6ff;
    --color-primary-100: #dbeafe;
    --color-primary-200: #bfdbfe;
    --color-primary-300: #93c5fd;
    --color-primary-400: #60a5fa;
    --color-primary-500: #3b82f6;
    --color-primary-600: #2563eb;
    --color-primary-700: #1d4ed8;
    --color-primary-800: #1e40af;
    --color-primary-900: #1e3a8a;
    
    /* Neutral grays */
    --color-gray-0: #ffffff;
    --color-gray-50: #f9fafb;
    --color-gray-100: #f3f4f6;
    --color-gray-200: #e5e7eb;
    --color-gray-300: #d1d5db;
    --color-gray-400: #9ca3af;
    --color-gray-500: #6b7280;
    --color-gray-600: #4b5563;
    --color-gray-700: #374151;
    --color-gray-800: #1f2937;
    --color-gray-900: #111827;
    
    /* Semantic colors */
    --color-success-50: #f0fdf4;
    --color-success-100: #dcfce7;
    --color-success-500: #22c55e;
    --color-success-600: #16a34a;
    --color-success-700: #15803d;
    
    --color-warning-50: #fffbeb;
    --color-warning-100: #fef3c7;
    --color-warning-500: #f59e0b;
    --color-warning-600: #d97706;
    --color-warning-700: #b45309;
    
    --color-error-50: #fef2f2;
    --color-error-100: #fee2e2;
    --color-error-500: #ef4444;
    --color-error-600: #dc2626;
    --color-error-700: #b91c1c;
    
    --color-info-50: #f0f9ff;
    --color-info-100: #e0f2fe;
    --color-info-500: #0ea5e9;
    --color-info-600: #0284c7;
    --color-info-700: #0369a1;
    
    /* ========================================
       SEAL STATE COLORS
       ======================================== */
    
    /* Seal is active and available */
    --seal-active-bg: var(--color-success-50);
    --seal-active-border: var(--color-success-500);
    --seal-active-text: var(--color-success-700);
    --seal-active-dot: #10b981;
    
    /* Seal is pending confirmation */
    --seal-pending-bg: var(--color-warning-50);
    --seal-pending-border: var(--color-warning-500);
    --seal-pending-text: var(--color-warning-700);
    --seal-pending-dot: #f59e0b;
    --seal-pending-pulse: rgba(245, 158, 11, 0.4);
    
    /* Seal has been consumed */
    --seal-consumed-bg: var(--color-gray-100);
    --seal-consumed-border: var(--color-gray-400);
    --seal-consumed-text: var(--color-gray-600);
    --seal-consumed-dot: #9ca3af;
    
    /* Seal is locked in a cross-chain transfer */
    --seal-locked-bg: var(--color-info-50);
    --seal-locked-border: var(--color-info-500);
    --seal-locked-text: var(--color-info-700);
    --seal-locked-dot: #0ea5e9;
    
    /* Seal has failed or is invalid */
    --seal-error-bg: var(--color-error-50);
    --seal-error-border: var(--color-error-500);
    --seal-error-text: var(--color-error-700);
    --seal-error-dot: #ef4444;

    /* ========================================
       SINGLE-USE SEAL SEMANTIC TOKENS
       Core UI vocabulary for seal states
       ======================================== */

    /* Seal is valid and unspent */
    --seal-unconsumed: #22c55e;
    /* Seal is spent, right transferred */
    --seal-consumed: #6b7280;
    /* Protocol violation detected */
    --seal-double-spent: #ef4444;
    /* Awaiting chain finality */
    --seal-pending: #f59e0b;

    /* ========================================
       PROOF VERIFICATION STATES
       ======================================== */

    --proof-valid: #22c55e;
    --proof-invalid: #ef4444;
    --proof-pending: #f59e0b;
    --proof-unverified: #6b7280;

    /* ========================================
       COMMITMENT CHAIN VISUALIZATION
       ======================================== */

    /* Genesis commitment */
    --chain-genesis: #8b5cf6;
    /* Intermediate commitment */
    --chain-node: #3b82f6;
    /* Latest valid commitment */
    --chain-latest: #22c55e;
    /* Hash chain link lines */
    --chain-link: #374151;

    /* ========================================
       SPACING SCALE
       ======================================== */
    --space-0: 0;
    --space-1: 0.25rem;   /* 4px */
    --space-2: 0.5rem;    /* 8px */
    --space-3: 0.75rem;   /* 12px */
    --space-4: 1rem;      /* 16px */
    --space-5: 1.25rem;   /* 20px */
    --space-6: 1.5rem;    /* 24px */
    --space-8: 2rem;      /* 32px */
    --space-10: 2.5rem;   /* 40px */
    --space-12: 3rem;     /* 48px */
    --space-16: 4rem;     /* 64px */
    --space-20: 5rem;     /* 80px */
    
    /* ========================================
       TYPOGRAPHY
       ======================================== */
    --font-sans: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    --font-mono: ui-monospace, SFMono-Regular, "SF Mono", Consolas, "Liberation Mono", Menlo, monospace;
    
    --text-xs: 0.75rem;   /* 12px */
    --text-sm: 0.875rem;  /* 14px */
    --text-base: 1rem;    /* 16px */
    --text-lg: 1.125rem;  /* 18px */
    --text-xl: 1.25rem;   /* 20px */
    --text-2xl: 1.5rem;   /* 24px */
    --text-3xl: 1.875rem; /* 30px */
    
    --font-normal: 400;
    --font-medium: 500;
    --font-semibold: 600;
    --font-bold: 700;
    
    --leading-tight: 1.25;
    --leading-normal: 1.5;
    --leading-relaxed: 1.625;
    
    /* ========================================
       BORDERS & SHADOWS
       ======================================== */
    --radius-none: 0;
    --radius-sm: 0.125rem;  /* 2px */
    --radius-md: 0.375rem;  /* 6px */
    --radius-lg: 0.5rem;    /* 8px */
    --radius-xl: 0.75rem;   /* 12px */
    --radius-2xl: 1rem;     /* 16px */
    --radius-full: 9999px;
    
    --shadow-sm: 0 1px 2px 0 rgba(0, 0, 0, 0.05);
    --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -2px rgba(0, 0, 0, 0.1);
    --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.1), 0 4px 6px -4px rgba(0, 0, 0, 0.1);
    --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 8px 10px -6px rgba(0, 0, 0, 0.1);
    
    /* Hash display specific */
    --hash-font: var(--font-mono);
    --hash-color: var(--color-gray-600);
    --hash-hover: var(--color-primary-600);
    --hash-short-length: 8;
}
"#;

/// Get CSS variable value as a string.
pub fn var(name: &str) -> String {
    format!("var(--{})", name)
}

/// Seal state CSS class generator.
pub fn seal_state_class(state: SealState) -> String {
    match state {
        SealState::Active => "seal-active".to_string(),
        SealState::Pending => "seal-pending".to_string(),
        SealState::Consumed => "seal-consumed".to_string(),
        SealState::Locked => "seal-locked".to_string(),
        SealState::Error => "seal-error".to_string(),
    }
}

/// Seal states for visual representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealState {
    /// Seal is active and available for use.
    Active,
    /// Seal is pending confirmation.
    Pending,
    /// Seal has been consumed.
    Consumed,
    /// Seal is locked in a cross-chain transfer.
    Locked,
    /// Seal operation failed.
    Error,
}

impl SealState {
    /// Get display label for the state.
    pub fn label(&self) -> &'static str {
        match self {
            SealState::Active => "Active",
            SealState::Pending => "Pending",
            SealState::Consumed => "Consumed",
            SealState::Locked => "Locked",
            SealState::Error => "Error",
        }
    }

    /// Get CSS color variable for the state dot.
    pub fn dot_color(&self) -> String {
        match self {
            SealState::Active => "var(--seal-active-dot)".to_string(),
            SealState::Pending => "var(--seal-pending-dot)".to_string(),
            SealState::Consumed => "var(--seal-consumed-dot)".to_string(),
            SealState::Locked => "var(--seal-locked-dot)".to_string(),
            SealState::Error => "var(--seal-error-dot)".to_string(),
        }
    }

    /// Check if state is actionable.
    pub fn is_actionable(&self) -> bool {
        matches!(self, SealState::Active | SealState::Pending)
    }
}
