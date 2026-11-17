use crate::Renderer;
use target_lexicon::Triple;

use crate::BuildRequest;

impl BuildRequest {
    pub(crate) fn renderer_enabled_by_dioxus_dependency(
        package: &krates::cm::Package,
    ) -> Option<(Renderer, String)> {
        let mut renderers = vec![];

        // Attempt to discover the platform directly from the dioxus dependency
        //
        // [dependencies]
        // dioxus = { features = ["web"] }
        //
        if let Some(dxs) = package.dependencies.iter().find(|dep| dep.name == "dioxus") {
            for feature in dxs.features.iter() {
                if let Some(renderer) = Renderer::autodetect_from_cargo_feature(feature) {
                    renderers.push((renderer, format!("dioxus/{}", feature)));
                }
            }
        }

        if renderers.len() != 1 {
            return None;
        }

        Some(renderers[0].clone())
    }

    pub(crate) fn features_that_enable_renderers(
        package: &krates::cm::Package,
    ) -> Vec<(Renderer, String)> {
        package
            .features
            .keys()
            .filter_map(|key| {
                Renderer::autodetect_from_cargo_feature(key).map(|v| (v, key.to_string()))
            })
            .collect()
    }

    /// Return the platforms that are enabled for the package only from the default features
    ///
    /// Ideally only one platform is enabled but we need to be able to
    pub(crate) fn enabled_cargo_toml_default_features_renderers(
        package: &krates::cm::Package,
    ) -> Vec<(Renderer, String)> {
        let mut renderers = vec![];

        // Start searching through the default features
        //
        // [features]
        // default = ["dioxus/web"]
        //
        // or
        //
        // [features]
        // default = ["web"]
        // web = ["dioxus/web"]
        let Some(default) = package.features.get("default") else {
            return renderers;
        };

        // we only trace features 1 level deep..
        // TODO: trace all enabled features, not just default features
        for feature in default.iter() {
            // If the user directly specified a platform we can just use that.
            if feature.starts_with("dioxus/") {
                let dx_feature = feature.trim_start_matches("dioxus/");
                let auto = Renderer::autodetect_from_cargo_feature(dx_feature);
                if let Some(auto) = auto {
                    renderers.push((auto, dx_feature.to_string()));
                }
            }

            // If the user is specifying an internal feature that points to a platform, we can use that
            let internal_feature = package.features.get(feature);
            if let Some(internal_feature) = internal_feature {
                for feature in internal_feature {
                    if feature.starts_with("dioxus/") {
                        let dx_feature = feature.trim_start_matches("dioxus/");
                        let auto = Renderer::autodetect_from_cargo_feature(dx_feature);
                        if let Some(auto) = auto {
                            renderers.push((auto, dx_feature.to_string()));
                        }
                    }
                }
            }
        }

        renderers.sort();
        renderers.dedup();

        renderers
    }

    /// Gather the features that are enabled for the package
    pub fn rendererless_features(package: &krates::cm::Package) -> Vec<String> {
        let Some(default) = package.features.get("default") else {
            return Vec::new();
        };

        let mut kept_features = vec![];

        // Only keep the top-level features in the default list that don't point to a platform directly
        // IE we want to drop `web` if default = ["web"]
        'top: for feature in default {
            // Don't keep features that point to a platform via dioxus/blah
            if feature.starts_with("dioxus/") {
                let dx_feature = feature.trim_start_matches("dioxus/");
                if Renderer::autodetect_from_cargo_feature(dx_feature).is_some() {
                    tracing::debug!(
                        "Dropping feature {feature} since it points to a platform renderer"
                    );
                    continue 'top;
                }
            }

            // Don't keep features that point to a platform via an internal feature
            if let Some(internal_feature) = package.features.get(feature) {
                for feature in internal_feature {
                    if feature.starts_with("dioxus/") {
                        let dx_feature = feature.trim_start_matches("dioxus/");
                        if Renderer::autodetect_from_cargo_feature(dx_feature).is_some() {
                            tracing::debug!(
                                "Dropping feature {feature} since it points to a platform renderer transitively"
                            );
                            continue 'top;
                        }
                    }
                }
            }

            // Otherwise we can keep it
            kept_features.push(feature.to_string());
        }

        kept_features
    }

    /// Get the features required to build for the given platform
    pub fn feature_for_platform_and_renderer(
        package: &krates::cm::Package,
        triple: &Triple,
        renderer: Renderer,
    ) -> Option<String> {
        // Try to find the feature that activates the dioxus feature for the given platform
        let dioxus_feature = renderer.feature_name(triple);

        let res = package.features.iter().find_map(|(key, features)| {
            // if the feature is just the name of the platform, we use that
            if key == dioxus_feature {
                tracing::debug!("Found feature {key} for renderer {renderer}");
                return Some(key.clone());
            }

            // Otherwise look for the feature that starts with dioxus/ or dioxus?/ and matches just the single platform
            // we are looking for.
            let mut dioxus_renderers_enabled = Vec::new();
            for feature in features {
                if let Some((_, after_dioxus)) = feature.split_once("dioxus") {
                    if let Some(dioxus_feature_enabled) =
                        after_dioxus.trim_start_matches('?').strip_prefix('/')
                    {
                        if Renderer::autodetect_from_cargo_feature(dioxus_feature_enabled).is_some()
                        {
                            dioxus_renderers_enabled.push(dioxus_feature_enabled.to_string());
                        }
                    }
                }
            }

            // If there is exactly one renderer enabled by this feature, we can use it
            if let [feature_name] = dioxus_renderers_enabled.as_slice() {
                if feature_name == dioxus_feature {
                    tracing::debug!(
                        "Found feature {key} for renderer {renderer} which enables dioxus/{renderer}"
                    );
                    return Some(key.clone());
                }
            }

            None
        });

        res.or_else(|| {
            let depends_on_dioxus = package.dependencies.iter().any(|dep| dep.name == "dioxus");
            if depends_on_dioxus {
                let fallback = format!("dioxus/{dioxus_feature}");
                tracing::debug!(
                    "Could not find explicit feature for renderer {renderer}, passing `fallback` instead"
                );
                Some(fallback)
            } else {
                None
            }
        })
    }
}
