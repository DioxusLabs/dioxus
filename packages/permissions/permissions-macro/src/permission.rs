use quote::{quote, ToTokens};
use std::hash::{DefaultHasher, Hash, Hasher};
use syn::{
    parse::{Parse, ParseStream},
    Token,
};

use permissions_core::{LocationPrecision, PermissionKind};

/// Parser for the permission!() macro syntax
pub struct PermissionParser {
    /// The permission kind being declared
    kind: PermissionKindParser,
    /// The user-facing description
    description: String,
}

impl Parse for PermissionParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse the permission kind
        let kind = input.parse::<PermissionKindParser>()?;

        // Parse the comma separator
        let _comma = input.parse::<Token![,]>()?;

        // Parse the description keyword
        let _description_keyword = input.parse::<syn::Ident>()?;
        if _description_keyword != "description" {
            return Err(syn::Error::new(
                _description_keyword.span(),
                "Expected 'description' keyword",
            ));
        }

        // Parse the equals sign
        let _equals = input.parse::<Token![=]>()?;

        // Parse the description string
        let description_lit = input.parse::<syn::LitStr>()?;
        let description = description_lit.value();

        Ok(Self {
            kind: kind.into(),
            description,
        })
    }
}

impl ToTokens for PermissionParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        // Generate the kind expression tokens directly
        let kind_tokens = self.kind.to_token_stream();
        let description = &self.description;

        // Generate a hash for unique symbol naming
        let mut hash = DefaultHasher::new();
        self.kind.hash(&mut hash);
        self.description.hash(&mut hash);
        let permission_hash = format!("{:016x}", hash.finish());

        // Check if this is a Custom permission
        let is_custom = matches!(self.kind, PermissionKindParser::Custom { .. });

        if is_custom {
            // For Custom permissions, skip serialization due to buffer size limitations
            // and just create the permission directly
            tokens.extend(quote! {
                {
                    // Create the permission instance directly for Custom permissions
                    permissions_core::Permission::new(
                        #kind_tokens,
                        #description,
                    )
                }
            });
        } else {
            // For regular permissions, use the normal serialization approach
            let link_section =
                crate::linker::generate_link_section(quote!(__PERMISSION), &permission_hash);

            tokens.extend(quote! {
                {
                    // Create the permission instance
                    const __PERMISSION: permissions_core::Permission = permissions_core::Permission::new(
                        #kind_tokens,
                        #description,
                    );

                    #link_section

                    // Force reference to prevent dead code elimination
                    static __REFERENCE_TO_LINK_SECTION: &'static [u8] = &__LINK_SECTION;

                    // Return the actual permission (not from embedded data for now)
                    __PERMISSION
                }
            });
        }
    }
}

/// Parser for permission kinds in the macro syntax
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum PermissionKindParser {
    Camera,
    Location(LocationPrecision),
    Microphone,
    PhotoLibrary,
    Contacts,
    Calendar,
    Bluetooth,
    Notifications,
    FileSystem,
    Network,
    Sms,
    PhoneState,
    PhoneCall,
    SystemAlertWindow,
    UserTracking,
    FaceId,
    LocalNetwork,
    Appointments,
    WindowsPhoneCall,
    EnterpriseAuth,
    Clipboard,
    Payment,
    ScreenWakeLock,
    Custom {
        android: String,
        ios: String,
        macos: String,
        windows: String,
        linux: String,
        web: String,
    },
}

impl Parse for PermissionKindParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<syn::Ident>()?;
        let name = ident.to_string();

        match name.as_str() {
            "Camera" => Ok(Self::Camera),
            "Location" => {
                // Parse Location(Fine) or Location(Coarse)
                let content;
                syn::parenthesized!(content in input);
                let precision_ident = content.parse::<syn::Ident>()?;

                match precision_ident.to_string().as_str() {
                    "Fine" => Ok(Self::Location(LocationPrecision::Fine)),
                    "Coarse" => Ok(Self::Location(LocationPrecision::Coarse)),
                    _ => Err(syn::Error::new(
                        precision_ident.span(),
                        "Expected 'Fine' or 'Coarse' for Location precision",
                    )),
                }
            }
            "Microphone" => Ok(Self::Microphone),
            "PhotoLibrary" => Ok(Self::PhotoLibrary),
            "Contacts" => Ok(Self::Contacts),
            "Calendar" => Ok(Self::Calendar),
            "Bluetooth" => Ok(Self::Bluetooth),
            "Notifications" => Ok(Self::Notifications),
            "FileSystem" => Ok(Self::FileSystem),
            "Network" => Ok(Self::Network),
            "Sms" => Ok(Self::Sms),
            "PhoneState" => Ok(Self::PhoneState),
            "PhoneCall" => Ok(Self::PhoneCall),
            "SystemAlertWindow" => Ok(Self::SystemAlertWindow),
            "UserTracking" => Ok(Self::UserTracking),
            "FaceId" => Ok(Self::FaceId),
            "LocalNetwork" => Ok(Self::LocalNetwork),
            "Appointments" => Ok(Self::Appointments),
            "WindowsPhoneCall" => Ok(Self::WindowsPhoneCall),
            "EnterpriseAuth" => Ok(Self::EnterpriseAuth),
            "Clipboard" => Ok(Self::Clipboard),
            "Payment" => Ok(Self::Payment),
            "ScreenWakeLock" => Ok(Self::ScreenWakeLock),
            "Custom" => {
                // Parse Custom { android = "...", ios = "...", ... }
                let content;
                syn::braced!(content in input);

                let mut android = String::new();
                let mut ios = String::new();
                let mut macos = String::new();
                let mut windows = String::new();
                let mut linux = String::new();
                let mut web = String::new();

                while !content.is_empty() {
                    let field_ident = content.parse::<syn::Ident>()?;
                    let _colon = content.parse::<syn::Token![=]>()?;
                    let field_value = content.parse::<syn::LitStr>()?;
                    let _comma = content.parse::<Option<syn::Token![,]>>()?;

                    match field_ident.to_string().as_str() {
                        "android" => android = field_value.value(),
                        "ios" => ios = field_value.value(),
                        "macos" => macos = field_value.value(),
                        "windows" => windows = field_value.value(),
                        "linux" => linux = field_value.value(),
                        "web" => web = field_value.value(),
                        _ => {
                            return Err(syn::Error::new(
                                field_ident.span(),
                                "Unknown field in Custom permission",
                            ));
                        }
                    }
                }

                Ok(Self::Custom {
                    android,
                    ios,
                    macos,
                    windows,
                    linux,
                    web,
                })
            }
            _ => Err(syn::Error::new(
                ident.span(),
                format!("Unknown permission kind: {}", name),
            )),
        }
    }
}

impl ToTokens for PermissionKindParser {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let kind_tokens = match self {
            PermissionKindParser::Camera => quote!(permissions_core::PermissionKind::Camera),
            PermissionKindParser::Location(precision) => {
                let precision_tokens = match precision {
                    LocationPrecision::Fine => quote!(permissions_core::LocationPrecision::Fine),
                    LocationPrecision::Coarse => {
                        quote!(permissions_core::LocationPrecision::Coarse)
                    }
                };
                quote!(permissions_core::PermissionKind::Location(#precision_tokens))
            }
            PermissionKindParser::Microphone => {
                quote!(permissions_core::PermissionKind::Microphone)
            }
            PermissionKindParser::PhotoLibrary => {
                quote!(permissions_core::PermissionKind::PhotoLibrary)
            }
            PermissionKindParser::Contacts => quote!(permissions_core::PermissionKind::Contacts),
            PermissionKindParser::Calendar => quote!(permissions_core::PermissionKind::Calendar),
            PermissionKindParser::Bluetooth => quote!(permissions_core::PermissionKind::Bluetooth),
            PermissionKindParser::Notifications => {
                quote!(permissions_core::PermissionKind::Notifications)
            }
            PermissionKindParser::FileSystem => {
                quote!(permissions_core::PermissionKind::FileSystem)
            }
            PermissionKindParser::Network => quote!(permissions_core::PermissionKind::Network),
            PermissionKindParser::Sms => quote!(permissions_core::PermissionKind::Sms),
            PermissionKindParser::PhoneState => {
                quote!(permissions_core::PermissionKind::PhoneState)
            }
            PermissionKindParser::PhoneCall => quote!(permissions_core::PermissionKind::PhoneCall),
            PermissionKindParser::SystemAlertWindow => {
                quote!(permissions_core::PermissionKind::SystemAlertWindow)
            }
            PermissionKindParser::UserTracking => {
                quote!(permissions_core::PermissionKind::UserTracking)
            }
            PermissionKindParser::FaceId => quote!(permissions_core::PermissionKind::FaceId),
            PermissionKindParser::LocalNetwork => {
                quote!(permissions_core::PermissionKind::LocalNetwork)
            }
            PermissionKindParser::Appointments => {
                quote!(permissions_core::PermissionKind::Appointments)
            }
            PermissionKindParser::WindowsPhoneCall => {
                quote!(permissions_core::PermissionKind::WindowsPhoneCall)
            }
            PermissionKindParser::EnterpriseAuth => {
                quote!(permissions_core::PermissionKind::EnterpriseAuth)
            }
            PermissionKindParser::Clipboard => quote!(permissions_core::PermissionKind::Clipboard),
            PermissionKindParser::Payment => quote!(permissions_core::PermissionKind::Payment),
            PermissionKindParser::ScreenWakeLock => {
                quote!(permissions_core::PermissionKind::ScreenWakeLock)
            }
            PermissionKindParser::Custom {
                android,
                ios,
                macos,
                windows,
                linux,
                web,
            } => quote!(permissions_core::PermissionKind::Custom {
                android: permissions_core::ConstStr::new(#android),
                ios: permissions_core::ConstStr::new(#ios),
                macos: permissions_core::ConstStr::new(#macos),
                windows: permissions_core::ConstStr::new(#windows),
                linux: permissions_core::ConstStr::new(#linux),
                web: permissions_core::ConstStr::new(#web),
            }),
        };
        tokens.extend(kind_tokens);
    }
}

impl From<PermissionKindParser> for PermissionKind {
    fn from(parser: PermissionKindParser) -> Self {
        match parser {
            PermissionKindParser::Camera => PermissionKind::Camera,
            PermissionKindParser::Location(precision) => PermissionKind::Location(precision),
            PermissionKindParser::Microphone => PermissionKind::Microphone,
            PermissionKindParser::PhotoLibrary => PermissionKind::PhotoLibrary,
            PermissionKindParser::Contacts => PermissionKind::Contacts,
            PermissionKindParser::Calendar => PermissionKind::Calendar,
            PermissionKindParser::Bluetooth => PermissionKind::Bluetooth,
            PermissionKindParser::Notifications => PermissionKind::Notifications,
            PermissionKindParser::FileSystem => PermissionKind::FileSystem,
            PermissionKindParser::Network => PermissionKind::Network,
            PermissionKindParser::Sms => PermissionKind::Sms,
            PermissionKindParser::PhoneState => PermissionKind::PhoneState,
            PermissionKindParser::PhoneCall => PermissionKind::PhoneCall,
            PermissionKindParser::SystemAlertWindow => PermissionKind::SystemAlertWindow,
            PermissionKindParser::UserTracking => PermissionKind::UserTracking,
            PermissionKindParser::FaceId => PermissionKind::FaceId,
            PermissionKindParser::LocalNetwork => PermissionKind::LocalNetwork,
            PermissionKindParser::Appointments => PermissionKind::Appointments,
            PermissionKindParser::WindowsPhoneCall => PermissionKind::WindowsPhoneCall,
            PermissionKindParser::EnterpriseAuth => PermissionKind::EnterpriseAuth,
            PermissionKindParser::Clipboard => PermissionKind::Clipboard,
            PermissionKindParser::Payment => PermissionKind::Payment,
            PermissionKindParser::ScreenWakeLock => PermissionKind::ScreenWakeLock,
            PermissionKindParser::Custom {
                android,
                ios,
                macos,
                windows,
                linux,
                web,
            } => PermissionKind::Custom {
                android: permissions_core::ConstStr::new(&android),
                ios: permissions_core::ConstStr::new(&ios),
                macos: permissions_core::ConstStr::new(&macos),
                windows: permissions_core::ConstStr::new(&windows),
                linux: permissions_core::ConstStr::new(&linux),
                web: permissions_core::ConstStr::new(&web),
            },
        }
    }
}
