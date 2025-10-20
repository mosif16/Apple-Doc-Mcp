use once_cell::sync::Lazy;
use std::collections::HashMap;

pub struct IntegrationLink {
    pub framework: &'static str,
    pub title: &'static str,
    pub path: &'static str,
    pub note: &'static str,
}

#[derive(Clone)]
pub struct RelatedItem {
    pub title: &'static str,
    pub path: &'static str,
    pub note: &'static str,
}

pub struct KnowledgeEntry {
    pub quick_tip: Option<&'static str>,
    pub related: &'static [RelatedItem],
    pub integration: &'static [IntegrationLink],
    pub snippet: Option<Snippet>,
}

#[derive(Clone, Copy)]
pub struct Snippet {
    pub language: &'static str,
    pub code: &'static str,
    pub caption: Option<&'static str>,
}

pub struct RecipeDefinition {
    pub id: &'static str,
    pub technology: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub keywords: &'static [&'static str],
    pub steps: &'static [&'static str],
    pub references: &'static [RelatedItem],
}

impl RecipeDefinition {
    fn matches(&self, query: &str, technology: &str) -> bool {
        if !self.technology.eq_ignore_ascii_case(technology.trim()) {
            return false;
        }

        let normalized = query.to_lowercase();
        self.keywords
            .iter()
            .any(|keyword| normalized.contains(keyword.to_lowercase().as_str()))
    }
}

static KNOWLEDGE: Lazy<HashMap<&'static str, KnowledgeEntry>> = Lazy::new(|| {
    use IntegrationLink as Link;
    use KnowledgeEntry as Entry;
    use RelatedItem as Related;

    const SWIFTUI_SEARCHABLE_INTEGRATIONS: [Link; 2] = [
        Link {
            framework: "UIKit",
            title: "UISearchController",
            path: "/documentation/uikit/uisearchcontroller",
            note: "Embed when hosting SwiftUI search inside UIKit navigation stacks.",
        },
        Link {
            framework: "AppKit",
            title: "NSSearchToolbarItem",
            path: "/documentation/appkit/nssearchtoolbaritem",
            note: "Provide macOS toolbar search alongside SwiftUI content.",
        },
    ];
    const SWIFTUI_SEARCHABLE_RELATED: [Related; 2] = [
        Related {
            title: "searchSuggestions(_:)",
            path: "/documentation/swiftui/view/searchsuggestions(_:)",
            note: "Offer auto-complete suggestions as people type.",
        },
        Related {
            title: "searchScopes(_:scopes:)",
            path: "/documentation/swiftui/view/searchscopes(_:scopes:)",
            note: "Partition search results into categories.",
        },
    ];

    const SWIFTUI_TEXTFIELD_INTEGRATIONS: [Link; 2] = [
        Link {
            framework: "UIKit",
            title: "UITextField",
            path: "/documentation/UIKit/UITextField",
            note: "Leverage UIKit delegates when you need granular editing control.",
        },
        Link {
            framework: "AppKit",
            title: "NSTextField",
            path: "/documentation/AppKit/NSTextField",
            note: "Use for macOS-specific behaviors like formatter delegates.",
        },
    ];
    const SWIFTUI_TEXTFIELD_RELATED: [Related; 3] = [
        Related {
            title: "TextFieldStyle",
            path: "/documentation/swiftui/textfieldstyle",
            note: "Select styling presets that align with platform idioms.",
        },
        Related {
            title: "focused(_:equals:)",
            path: "/documentation/swiftui/view/focused(_:equals:)",
            note: "Manage focus programmatically for text inputs.",
        },
        Related {
            title: "TextInputAutocapitalization",
            path: "/documentation/swiftui/textinputautocapitalization",
            note: "Tune keyboard behavior for the field’s content.",
        },
    ];

    const SWIFTUI_TEXTFIELD_SNIPPET: Snippet = Snippet {
        language: "swift",
        code: "@State private var username = \"\"\n\nTextField(\"Username\", text: $username)\n    .textInputAutocapitalization(.never)\n    .textFieldStyle(.roundedBorder)",
        caption: Some("Bind text to state and customize keyboard behavior."),
    };

    const SWIFTUI_LIST_INTEGRATIONS: [Link; 1] = [Link {
        framework: "UIKit",
        title: "UITableView",
        path: "/documentation/uikit/uitableview",
        note: "Bridge to UIKit list controllers during incremental migration.",
    }];
    const SWIFTUI_LIST_RELATED: [Related; 3] = [
        Related {
            title: "refreshable(action:)",
            path: "/documentation/swiftui/view/refreshable(action:)",
            note: "Add pull-to-refresh to long lists.",
        },
        Related {
            title: "swipeActions(edge:allowsFullSwipe:content:)",
            path: "/documentation/swiftui/view/swipeactions(edge:allowsfullswipe:content:)",
            note: "Expose trailing actions that mirror UITableView behaviors.",
        },
        Related {
            title: "listRowSeparator(_:edges:)",
            path: "/documentation/swiftui/view/listrowseparator(_:edges:)",
            note: "Control separators for grouped or inset list styles.",
        },
    ];

    const SWIFTUI_LIST_SNIPPET: Snippet = Snippet {
        language: "swift",
        code: "List(filteredItems) { item in\n    Label(item.title, systemImage: item.icon)\n}\n.listStyle(.insetGrouped)",
        caption: Some("Filter data and apply a list style that matches the platform."),
    };

    const SWIFTUI_ACCESSIBILITY_RELATED: [Related; 2] = [
        Related {
            title: "accessibilityValue(_:)",
            path: "/documentation/swiftui/view/accessibilityvalue(_:)",
            note: "Describe dynamic values such as progress or selection.",
        },
        Related {
            title: "accessibilityHint(_:)",
            path: "/documentation/swiftui/view/accessibilityhint(_:)",
            note: "Explain the result of activating the element.",
        },
    ];

    const SWIFTUI_ACCESSIBILITY_SNIPPET: Snippet = Snippet {
        language: "swift",
        code: "Image(systemName: \"speaker.wave.2.fill\")\n    .accessibilityLabel(\"Playback volume\")\n    .accessibilityValue(\"70 percent\")\n    .accessibilityHint(\"Adjust with the volume buttons\")",
        caption: Some("Pair labels, values, and hints for richer VoiceOver output."),
    };

    const SWIFTUI_TEXT_SNIPPET: Snippet = Snippet {
        language: "swift",
        code: "Text(\"Welcome\")\n    .font(.title.bold())\n    .foregroundStyle(.primary)",
        caption: Some("Render static copy with typography that adapts to Dynamic Type."),
    };

    const SWIFTUI_SEARCH_TOPIC_SNIPPET: Snippet = Snippet {
        language: "swift",
        code: "@State private var searchText = \"\"\n\nNavigationStack {\n    List(results) { result in\n        Text(result.title)\n    }\n    .searchable(text: $searchText)\n    .searchSuggestions {\n        ForEach(suggestions) { suggestion in\n            Text(suggestion).searchCompletion(suggestion)\n        }\n    }\n}",
        caption: Some("Combine search field and suggestions inside navigation."),
    };

    const UIKIT_UITEXTFIELD_SNIPPET: Snippet = Snippet {
        language: "swift",
        code: "let textField = UITextField(frame: .zero)\ntextField.placeholder = \"Email address\"\ntextField.keyboardType = .emailAddress\ntextField.delegate = self",
        caption: Some("Configure delegates and text input traits for UIKit forms."),
    };

    const SWIFTUI_SEARCH_TOPIC_RELATED: [Related; 3] = [
        Related {
            title: "searchable(text:placement:prompt:)",
            path: "/documentation/swiftui/view/searchable(text:placement:prompt:)",
            note: "Enable the search field.",
        },
        Related {
            title: "searchSuggestions(_:)",
            path: "/documentation/swiftui/view/searchsuggestions(_:)",
            note: "Offer query completions.",
        },
        Related {
            title: "List",
            path: "/documentation/swiftui/list",
            note: "Display search results in scrollable content.",
        },
    ];

    const SWIFTUI_SEARCH_TOPIC_INTEGRATIONS: [Link; 1] = [Link {
        framework: "UIKit",
        title: "UISearchController",
        path: "/documentation/uikit/uisearchcontroller",
        note: "Embed SwiftUI search inside UIKit navigation stacks when iterating gradually.",
    }];

    const UIKIT_UITEXTFIELD_INTEGRATIONS: [Link; 1] = [Link {
        framework: "SwiftUI",
        title: "TextField",
        path: "/documentation/SwiftUI/TextField",
        note: "Embed SwiftUI text inputs with UIHostingController.",
    }];
    const UIKIT_UITEXTFIELD_RELATED: [Related; 2] = [
        Related {
            title: "UITextFieldDelegate",
            path: "/documentation/uikit/uitextfielddelegate",
            note: "Respond to editing events and validate input.",
        },
        Related {
            title: "UITextInputTraits",
            path: "/documentation/uikit/uitextinputtraits",
            note: "Customize keyboard and autocorrection behavior.",
        },
    ];

    let mut map = HashMap::new();
    map.insert(
        "swiftui::searchable(text:placement:prompt:)",
        Entry {
            quick_tip: Some("Pair with searchSuggestions(_:) and searchScopes(_:scopes:) to cover completions and scoped results."),
            related: &SWIFTUI_SEARCHABLE_RELATED,
            integration: &SWIFTUI_SEARCHABLE_INTEGRATIONS,
            snippet: Some(SWIFTUI_SEARCHABLE_SNIPPET),
        },
    );
    map.insert(
        "swiftui::textfield",
        Entry {
            quick_tip: Some(
                "Use modifiers like focused(_:equals:) to drive validation and submit actions.",
            ),
            related: &SWIFTUI_TEXTFIELD_RELATED,
            integration: &SWIFTUI_TEXTFIELD_INTEGRATIONS,
            snippet: Some(SWIFTUI_TEXTFIELD_SNIPPET),
        },
    );
    map.insert(
        "swiftui::list",
        Entry {
            quick_tip: Some("Adopt listStyle(_:) to align visuals with platform conventions."),
            related: &SWIFTUI_LIST_RELATED,
            integration: &SWIFTUI_LIST_INTEGRATIONS,
            snippet: Some(SWIFTUI_LIST_SNIPPET),
        },
    );
    map.insert(
        "swiftui::accessibilitylabel(_:)",
        Entry {
            quick_tip: Some("Combine with accessibilityHint(_:) to clarify the control’s result."),
            related: &SWIFTUI_ACCESSIBILITY_RELATED,
            integration: &[],
            snippet: Some(SWIFTUI_ACCESSIBILITY_SNIPPET),
        },
    );
    map.insert(
        "swiftui::text",
        Entry {
            quick_tip: Some("Prefer system fonts and styles for automatic Dynamic Type support."),
            related: &[
                Related {
                    title: "font(_:)",
                    path: "/documentation/swiftui/view/font(_:)",
                    note: "Apply semantic styles that adapt across platforms.",
                },
                Related {
                    title: "foregroundStyle(_:)",
                    path: "/documentation/swiftui/view/foregroundstyle(_:)",
                    note: "Use SF Symbols colors and gradients for emphasis.",
                },
            ],
            integration: &[
                Link {
                    framework: "UIKit",
                    title: "UILabel",
                    path: "/documentation/uikit/uilabel",
                    note: "Use when embedding SwiftUI text inside UIKit layouts.",
                },
                Link {
                    framework: "AppKit",
                    title: "NSTextField",
                    path: "/documentation/appkit/nstextfield",
                    note: "Bridge to macOS text controls in hybrid interfaces.",
                },
            ],
            snippet: Some(SWIFTUI_TEXT_SNIPPET),
        },
    );
    map.insert(
        "swiftui::search",
        Entry {
            quick_tip: Some("Use searchSuggestions(_:), searchScopes(_:scopes:), and tokens to shape the experience."),
            related: &SWIFTUI_SEARCH_TOPIC_RELATED,
            integration: &SWIFTUI_SEARCH_TOPIC_INTEGRATIONS,
            snippet: Some(SWIFTUI_SEARCH_TOPIC_SNIPPET),
        },
    );
    map.insert(
        "uikit::uitextfield",
        Entry {
            quick_tip: Some("Adopt UITextFieldDelegate for validation and formatting."),
            related: &UIKIT_UITEXTFIELD_RELATED,
            integration: &UIKIT_UITEXTFIELD_INTEGRATIONS,
            snippet: Some(UIKIT_UITEXTFIELD_SNIPPET),
        },
    );
    map
});

static RECIPES: Lazy<Vec<RecipeDefinition>> = Lazy::new(|| {
    use RecipeDefinition as Recipe;
    use RelatedItem as Related;

    const SEARCH_RECIPE_REFS: [Related; 3] = [
        Related {
            title: "Adding a search interface to your app",
            path: "/documentation/swiftui/adding-a-search-interface-to-your-app",
            note: "High-level walkthrough of the searchable modifier.",
        },
        Related {
            title: "searchable(text:placement:prompt:)",
            path: "/documentation/swiftui/view/searchable(text:placement:prompt:)",
            note: "Primary modifier for enabling search.",
        },
        Related {
            title: "List",
            path: "/documentation/swiftui/list",
            note: "Show search results within scrolling content.",
        },
    ];

    const SUGGESTIONS_RECIPE_REFS: [Related; 3] = [
        Related {
            title: "searchSuggestions(_:)",
            path: "/documentation/swiftui/view/searchsuggestions(_:)",
            note: "Provide completions underneath the search field.",
        },
        Related {
            title: "searchCompletion(_:)",
            path: "/documentation/swiftui/text/searchcompletion(_:)",
            note: "Associate display names with underlying values.",
        },
        Related {
            title: "searchable(text:tokens:suggestedTokens:placement:prompt:token:)",
            path: "/documentation/swiftui/view/searchable(text:tokens:suggestedtokens:placement:prompt:token:)",
            note: "Use token-based suggestions for structured filtering.",
        },
    ];

    const SCOPES_RECIPE_REFS: [Related; 2] = [
        Related {
            title: "searchScopes(_:scopes:)",
            path: "/documentation/swiftui/view/searchscopes(_:scopes:)",
            note: "Switch between categories in the search UI.",
        },
        Related {
            title: "SearchScope",
            path: "/documentation/swiftui/searchscope",
            note: "Define reusable scope identifiers.",
        },
    ];

    vec![
        Recipe {
            id: "swiftui-search-list",
            technology: "swiftui",
            title: "Add searchable support to a SwiftUI list",
            summary: "Wire a search field above a List and filter results reactively.",
            keywords: &[
                "how do i add search",
                "search list",
                "searchable list",
                "swiftui search list",
            ],
            steps: &[
                "Wrap your data in @State or @Observable so you can filter in-place.",
                "Add searchable(text:placement:prompt:) to the container that hosts the List.",
                "Filter the list items using the bound search text, ideally in a computed property.",
                "Provide empty-state content when the filtered results are empty.",
            ],
            references: &SEARCH_RECIPE_REFS,
        },
        Recipe {
            id: "swiftui-search-suggestions",
            technology: "swiftui",
            title: "Offer dynamic search suggestions",
            summary: "Display inline completions that users can tap to complete their search query.",
            keywords: &[
                "how do i add search suggestions",
                "search suggestions",
                "swifui suggestions",
                "searchcompletion",
                "search tokens",
            ],
            steps: &[
                "Maintain a lightweight suggestions array derived from recent searches or server hints.",
                "Call searchSuggestions(_:) within the searchable modifier to render completions.",
                "Use searchCompletion(_:) or tokens to map suggestion taps to structured values.",
                "Update the suggestions array in onChange(of:) to keep results relevant.",
            ],
            references: &SUGGESTIONS_RECIPE_REFS,
        },
        Recipe {
            id: "swiftui-search-scopes",
            technology: "swiftui",
            title: "Limit search results with scopes",
            summary: "Add segmented controls that keep the search query while filtering categories.",
            keywords: &[
                "how do i add search scope",
                "search scope",
                "searchScopes",
                "scope search",
            ],
            steps: &[
                "Define an enum that conforms to Hashable to represent each scope.",
                "Bind the selected scope to state and update the search predicate accordingly.",
                "Add searchScopes(_:scopes:) to describe the available categories.",
                "Adjust the search results view to react to both the text query and the selected scope.",
            ],
            references: &SCOPES_RECIPE_REFS,
        },
    ]
});

pub fn lookup(technology: &str, symbol_title: &str) -> Option<&'static KnowledgeEntry> {
    let key = format!(
        "{}::{}",
        technology.trim().to_lowercase(),
        symbol_title.trim().to_lowercase()
    );
    KNOWLEDGE.get(key.as_str())
}

pub fn find_recipe(technology: &str, query: &str) -> Option<&'static RecipeDefinition> {
    RECIPES
        .iter()
        .find(|recipe| recipe.matches(query, technology))
}

pub fn recipes_for(technology: &str) -> Vec<&'static RecipeDefinition> {
    RECIPES
        .iter()
        .filter(|recipe| recipe.technology.eq_ignore_ascii_case(technology))
        .collect()
}

pub fn snippet(entry: &KnowledgeEntry) -> Option<Snippet> {
    entry.snippet
}

pub fn related_items(entry: &KnowledgeEntry) -> &'static [RelatedItem] {
    entry.related
}

pub fn integration_links(entry: &KnowledgeEntry) -> &'static [IntegrationLink] {
    entry.integration
}
const SWIFTUI_SEARCHABLE_SNIPPET: Snippet = Snippet {
        language: "swift",
        code: "List(filteredBooks) { book in\n    Text(book.title)\n}\n.searchable(text: $query, placement: .navigationBarDrawer, prompt: \"Search books\")",
        caption: Some("Attach `searchable` to filter list content reactively."),
    };
