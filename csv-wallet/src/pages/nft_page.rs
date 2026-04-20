//! NFT Gallery page component

use dioxus::prelude::*;
use crate::context::{NftRecord, NftCollection, NftStatus};

#[component]
pub fn NftPage() -> Element {
    let mut selected_tab = use_signal(|| 0); // 0: NFTs, 1: Collections
    
    rsx! {
        div { class: "min-h-screen bg-gray-50 dark:bg-gray-900",
            div { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8",
                // Header
                div { class: "mb-8",
                    h1 { class: "text-3xl font-bold text-gray-900 dark:text-white mb-4",
                        "NFT Gallery"
                    }
                    
                    // Tab navigation
                    div { class: "border-b border-gray-200 dark:border-gray-700",
                        nav { class: "-mb-px flex space-x-8",
                            button {
                                class: if *selected_tab.read() == 0 {
                                    "py-2 px-1 border-b-2 border-indigo-500 font-medium text-sm text-indigo-600 dark:text-indigo-300"
                                } else {
                                    "py-2 px-1 border-b-2 border-transparent font-medium text-sm text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300"
                                },
                                onclick: move |_| selected_tab.set(0),
                                "My NFTs"
                            }
                            button {
                                class: if *selected_tab.read() == 1 {
                                    "py-2 px-1 border-b-2 border-indigo-500 font-medium text-sm text-indigo-600 dark:text-indigo-300"
                                } else {
                                    "py-2 px-1 border-b-2 border-transparent font-medium text-sm text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300"
                                },
                                onclick: move |_| selected_tab.set(1),
                                "Collections"
                            }
                        }
                    }
                }

                // Tab content
                div { class: "mt-8",
                    match *selected_tab.read() {
                        0 => rsx! { NftGallery {} },
                        1 => rsx! { NftCollections {} },
                        _ => rsx! { div { "Invalid tab" } }
                    }
                }
            }
        }
    }
}

#[component]
pub fn NftGallery() -> Element {
    // Mock NFT data for demonstration
    let mock_nfts = [NftRecord {
            id: "nft-1".to_string(),
            chain: csv_adapter_core::Chain::Ethereum,
            collection_id: Some("collection-1".to_string()),
            name: "Cosmic Ape #1234".to_string(),
            symbol: Some("APE".to_string()),
            description: Some("A rare cosmic ape exploring the metaverse".to_string()),
            owner: "0x1234...5678".to_string(),
            token_id: Some("1234".to_string()),
            metadata: None,
            image_url: Some("https://example.com/ape1.jpg".to_string()),
            external_url: Some("https://opensea.io/assets/ethereum/0x.../1234".to_string()),
            created_at: 1640995200,
            status: NftStatus::Owned,
        },
        NftRecord {
            id: "nft-2".to_string(),
            chain: csv_adapter_core::Chain::Solana,
            collection_id: Some("collection-2".to_string()),
            name: "Solana Monkey #567".to_string(),
            symbol: Some("SOLMONK".to_string()),
            description: Some("A cool monkey from the Solana ecosystem".to_string()),
            owner: "9WzDXwBbmkg8ZXtBjTfZFpK9S".to_string(),
            token_id: Some("567".to_string()),
            metadata: None,
            image_url: Some("https://example.com/monkey1.jpg".to_string()),
            external_url: Some("https://magiceden.io/items/567".to_string()),
            created_at: 1640995200,
            status: NftStatus::Owned,
        }];

    rsx! {
        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6",
            {mock_nfts.iter().map(|nft| {
                rsx! {
                    NftCard { nft: nft.clone() }
                }
            })}
        }
    }
}

#[component]
pub fn NftCollections() -> Element {
    // Mock collection data
    let mock_collections = [NftCollection {
            id: "collection-1".to_string(),
            chain: csv_adapter_core::Chain::Ethereum,
            name: "Bored Ape Yacht Club".to_string(),
            symbol: "BAYC".to_string(),
            description: Some("A collection of unique apes".to_string()),
            image_url: Some("https://example.com/bayc.jpg".to_string()),
            external_url: Some("https://boredapeyachtclub.com".to_string()),
            total_supply: 10000,
            owner_count: 6500,
            floor_price: Some(15.5),
            created_at: 1640995200,
        },
        NftCollection {
            id: "collection-2".to_string(),
            chain: csv_adapter_core::Chain::Solana,
            name: "Solana Monkey Business".to_string(),
            symbol: "SMB".to_string(),
            description: Some("Monkeys building on Solana".to_string()),
            image_url: Some("https://example.com/smb.jpg".to_string()),
            external_url: Some("https://smb.genesisdao.com".to_string()),
            total_supply: 5000,
            owner_count: 1200,
            floor_price: Some(2.8),
            created_at: 1640995200,
        }];

    rsx! {
        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6",
            {mock_collections.iter().map(|collection| {
                rsx! {
                    CollectionCard { collection: collection.clone() }
                }
            })}
        }
    }
}

#[component]
pub fn NftDetail(id: String) -> Element {
    rsx! {
        div { class: "min-h-screen bg-gray-50 dark:bg-gray-900",
            div { class: "max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8",
                div { class: "bg-white dark:bg-gray-800 rounded-lg shadow-md p-6",
                    h1 { class: "text-2xl font-bold text-gray-900 dark:text-white mb-4",
                        "NFT Details"
                    }
                    p { class: "text-gray-600 dark:text-gray-400",
                        "NFT ID: {id}"
                    }
                }
            }
        }
    }
}

#[component]
fn NftCard(nft: NftRecord) -> Element {
    rsx! {
        div { class: "bg-white dark:bg-gray-800 rounded-lg shadow-md overflow-hidden hover:shadow-lg transition-shadow duration-200",
            // NFT Image
            div { class: "relative h-48 bg-gray-200 dark:bg-gray-700",
                if let Some(image_url) = &nft.image_url {
                    img {
                        src: "{image_url}",
                        alt: "{nft.name}",
                        class: "w-full h-full object-cover"
                    }
                } else {
                    div { class: "w-full h-full flex items-center justify-center",
                        div { class: "text-gray-400 dark:text-gray-500 text-4xl",
                            "🖼️"
                        }
                    }
                }
                
                // Chain badge
                div { class: "absolute top-2 right-2",
                    span { class: "px-2 py-1 text-xs font-semibold rounded-full bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
                        "{nft.chain}"
                    }
                }
            }
            
            // NFT Info
            div { class: "p-4",
                div { class: "flex items-start justify-between",
                    h3 { class: "text-lg font-semibold text-gray-900 dark:text-white truncate",
                        "{nft.name}"
                    }
                    span { class: "px-2 py-1 text-xs font-medium rounded-full {get_status_color(&nft.status)}",
                        "{nft.status}"
                    }
                }
                
                if let Some(symbol) = &nft.symbol {
                    p { class: "text-sm text-gray-600 dark:text-gray-400 mt-1",
                        "Symbol: {symbol}"
                    }
                }
                
                if let Some(collection_id) = &nft.collection_id {
                    p { class: "text-sm text-gray-600 dark:text-gray-400 mt-1",
                        "Collection: {collection_id}"
                    }
                }
                
                p { class: "text-xs text-gray-500 dark:text-gray-500 mt-2",
                    "Owner: {nft.owner}"
                }
                
                if let Some(external_url) = &nft.external_url {
                    a {
                        href: "{external_url}",
                        target: "_blank",
                        class: "mt-3 inline-flex items-center px-3 py-2 border border-transparent text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500",
                        "View on Marketplace"
                    }
                }
            }
        }
    }
}

#[component]
fn CollectionCard(collection: NftCollection) -> Element {
    rsx! {
        div { class: "bg-white dark:bg-gray-800 rounded-lg shadow-md overflow-hidden hover:shadow-lg transition-shadow duration-200",
            // Collection Image
            div { class: "relative h-48 bg-gray-200 dark:bg-gray-700",
                if let Some(image_url) = &collection.image_url {
                    img {
                        src: "{image_url}",
                        alt: "{collection.name}",
                        class: "w-full h-full object-cover"
                    }
                } else {
                    div { class: "w-full h-full flex items-center justify-center",
                        div { class: "text-gray-400 dark:text-gray-500 text-4xl",
                            "🖼️"
                        }
                    }
                }
                
                // Chain badge
                div { class: "absolute top-2 right-2",
                    span { class: "px-2 py-1 text-xs font-semibold rounded-full bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
                        "{collection.chain}"
                    }
                }
            }
            
            // Collection Info
            div { class: "p-4",
                div { class: "flex items-start justify-between",
                    h3 { class: "text-lg font-semibold text-gray-900 dark:text-white truncate",
                        "{collection.name}"
                    }
                    span { class: "px-2 py-1 text-xs font-medium rounded-full bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200",
                        "{collection.symbol}"
                    }
                }
                
                if let Some(description) = &collection.description {
                    p { class: "text-sm text-gray-600 dark:text-gray-400 mt-2 line-clamp-2",
                        "{description}"
                    }
                }
                
                div { class: "mt-3 space-y-1",
                    div { class: "flex justify-between text-sm",
                        span { class: "text-gray-500 dark:text-gray-400",
                            "Total Supply: {collection.total_supply}"
                        }
                        span { class: "text-gray-500 dark:text-gray-400",
                            "Owners: {collection.owner_count}"
                        }
                    }
                    
                    if let Some(floor_price) = &collection.floor_price {
                        div { class: "flex justify-between text-sm",
                            span { class: "text-gray-500 dark:text-gray-400",
                                "Floor Price"
                            }
                            span { class: "font-semibold text-gray-900 dark:text-white",
                                "Ξ {floor_price}"
                            }
                        }
                    }
                }
                
                if let Some(external_url) = &collection.external_url {
                    a {
                        href: "{external_url}",
                        target: "_blank",
                        class: "mt-3 inline-flex items-center px-3 py-2 border border-transparent text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500",
                        "View Collection"
                    }
                }
            }
        }
    }
}

fn get_status_color(status: &NftStatus) -> &'static str {
    match status {
        NftStatus::Owned => "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200",
        NftStatus::Transferred => "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200",
        NftStatus::Burned => "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200",
        NftStatus::Listed => "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
    }
}
