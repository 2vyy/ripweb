# Tokenizer Audit

This report compares candidate aggressive-mode transforms against the OpenAI `cl100k` tokenizer.

## Strategies

- `markdown`: Identity baseline.
- `aggressive_current`: Current aggressive mode.
- `drop_ui_lines`: Remove low-value UI-only lines such as copy affordances.
- `strip_heading_anchors`: Remove decorative heading anchor links.
- `label_only_internal_links`: Replace low-value internal relative links with labels only.
- `footnote_links`: Rewrite inline Markdown links to footnotes.

## Summary

| strategy | avg delta vs markdown | improved docs | worse docs |
| --- | ---: | ---: | ---: |
| markdown | 0.0 | 0 | 0 |
| aggressive_current | -241.6 | 7 | 1 |
| drop_ui_lines | 0.0 | 0 | 0 |
| strip_heading_anchors | -13.8 | 1 | 0 |
| label_only_internal_links | -208.7 | 6 | 4 |
| footnote_links | 308.2 | 0 | 12 |

## Per Document

| document | strategy | tokens | delta |
| --- | --- | ---: | ---: |
| shared_corpus / react_dev_usestate | markdown | 26739 | 0 |
| shared_corpus / react_dev_usestate | aggressive_current | 26551 | -188 |
| shared_corpus / react_dev_usestate | drop_ui_lines | 26739 | 0 |
| shared_corpus / react_dev_usestate | strip_heading_anchors | 26739 | 0 |
| shared_corpus / react_dev_usestate | label_only_internal_links | 26872 | 133 |
| shared_corpus / react_dev_usestate | footnote_links | 26835 | 96 |
| shared_corpus / stackoverflow_11227809 | markdown | 27048 | 0 |
| shared_corpus / stackoverflow_11227809 | aggressive_current | 25041 | -2007 |
| shared_corpus / stackoverflow_11227809 | drop_ui_lines | 27048 | 0 |
| shared_corpus / stackoverflow_11227809 | strip_heading_anchors | 27048 | 0 |
| shared_corpus / stackoverflow_11227809 | label_only_internal_links | 25056 | -1992 |
| shared_corpus / stackoverflow_11227809 | footnote_links | 28612 | 1564 |
| shared_corpus / docs_rs_axum | markdown | 3560 | 0 |
| shared_corpus / docs_rs_axum | aggressive_current | 3148 | -412 |
| shared_corpus / docs_rs_axum | drop_ui_lines | 3560 | 0 |
| shared_corpus / docs_rs_axum | strip_heading_anchors | 3395 | -165 |
| shared_corpus / docs_rs_axum | label_only_internal_links | 3173 | -387 |
| shared_corpus / docs_rs_axum | footnote_links | 4063 | 503 |
| shared_corpus / paulgraham_essay | markdown | 14414 | 0 |
| shared_corpus / paulgraham_essay | aggressive_current | 14326 | -88 |
| shared_corpus / paulgraham_essay | drop_ui_lines | 14414 | 0 |
| shared_corpus / paulgraham_essay | strip_heading_anchors | 14414 | 0 |
| shared_corpus / paulgraham_essay | label_only_internal_links | 14326 | -88 |
| shared_corpus / paulgraham_essay | footnote_links | 14631 | 217 |
| shared_corpus / mdnwebdocs_fetch | markdown | 4248 | 0 |
| shared_corpus / mdnwebdocs_fetch | aggressive_current | 4107 | -141 |
| shared_corpus / mdnwebdocs_fetch | drop_ui_lines | 4248 | 0 |
| shared_corpus / mdnwebdocs_fetch | strip_heading_anchors | 4248 | 0 |
| shared_corpus / mdnwebdocs_fetch | label_only_internal_links | 4112 | -136 |
| shared_corpus / mdnwebdocs_fetch | footnote_links | 4735 | 487 |
| shared_corpus / rustblog_post | markdown | 2207 | 0 |
| shared_corpus / rustblog_post | aggressive_current | 2132 | -75 |
| shared_corpus / rustblog_post | drop_ui_lines | 2207 | 0 |
| shared_corpus / rustblog_post | strip_heading_anchors | 2207 | 0 |
| shared_corpus / rustblog_post | label_only_internal_links | 2138 | -69 |
| shared_corpus / rustblog_post | footnote_links | 2336 | 129 |
| shared_corpus / devto_article | markdown | 2639 | 0 |
| shared_corpus / devto_article | aggressive_current | 2669 | 30 |
| shared_corpus / devto_article | drop_ui_lines | 2639 | 0 |
| shared_corpus / devto_article | strip_heading_anchors | 2639 | 0 |
| shared_corpus / devto_article | label_only_internal_links | 2674 | 35 |
| shared_corpus / devto_article | footnote_links | 2975 | 336 |
| freeze_review:irs_traditional_and_roth_iras / https://www.irs.gov/retirement-plans/traditional-and-roth-iras | markdown | 1188 | 0 |
| freeze_review:irs_traditional_and_roth_iras / https://www.irs.gov/retirement-plans/traditional-and-roth-iras | aggressive_current | 1188 | 0 |
| freeze_review:irs_traditional_and_roth_iras / https://www.irs.gov/retirement-plans/traditional-and-roth-iras | drop_ui_lines | 1188 | 0 |
| freeze_review:irs_traditional_and_roth_iras / https://www.irs.gov/retirement-plans/traditional-and-roth-iras | strip_heading_anchors | 1188 | 0 |
| freeze_review:irs_traditional_and_roth_iras / https://www.irs.gov/retirement-plans/traditional-and-roth-iras | label_only_internal_links | 1203 | 15 |
| freeze_review:irs_traditional_and_roth_iras / https://www.irs.gov/retirement-plans/traditional-and-roth-iras | footnote_links | 1276 | 88 |
| freeze_review:harvard_gut_health / https://www.health.harvard.edu/healthy-aging-and-longevity/5-simple-ways-to-improve-gut-health | markdown | 2519 | 0 |
| freeze_review:harvard_gut_health / https://www.health.harvard.edu/healthy-aging-and-longevity/5-simple-ways-to-improve-gut-health | aggressive_current | 2519 | 0 |
| freeze_review:harvard_gut_health / https://www.health.harvard.edu/healthy-aging-and-longevity/5-simple-ways-to-improve-gut-health | drop_ui_lines | 2519 | 0 |
| freeze_review:harvard_gut_health / https://www.health.harvard.edu/healthy-aging-and-longevity/5-simple-ways-to-improve-gut-health | strip_heading_anchors | 2519 | 0 |
| freeze_review:harvard_gut_health / https://www.health.harvard.edu/healthy-aging-and-longevity/5-simple-ways-to-improve-gut-health | label_only_internal_links | 2519 | 0 |
| freeze_review:harvard_gut_health / https://www.health.harvard.edu/healthy-aging-and-longevity/5-simple-ways-to-improve-gut-health | footnote_links | 2629 | 110 |
| freeze_review:texas_small_claims_filing / https://guides.sll.texas.gov/small-claims/filing-information | markdown | 2474 | 0 |
| freeze_review:texas_small_claims_filing / https://guides.sll.texas.gov/small-claims/filing-information | aggressive_current | 2456 | -18 |
| freeze_review:texas_small_claims_filing / https://guides.sll.texas.gov/small-claims/filing-information | drop_ui_lines | 2474 | 0 |
| freeze_review:texas_small_claims_filing / https://guides.sll.texas.gov/small-claims/filing-information | strip_heading_anchors | 2474 | 0 |
| freeze_review:texas_small_claims_filing / https://guides.sll.texas.gov/small-claims/filing-information | label_only_internal_links | 2456 | -18 |
| freeze_review:texas_small_claims_filing / https://guides.sll.texas.gov/small-claims/filing-information | footnote_links | 2579 | 105 |
| freeze_review:cdc_covid_vaccines_how_they_work / https://www.cdc.gov/covid/vaccines/how-they-work.html | markdown | 2495 | 0 |
| freeze_review:cdc_covid_vaccines_how_they_work / https://www.cdc.gov/covid/vaccines/how-they-work.html | aggressive_current | 2495 | 0 |
| freeze_review:cdc_covid_vaccines_how_they_work / https://www.cdc.gov/covid/vaccines/how-they-work.html | drop_ui_lines | 2495 | 0 |
| freeze_review:cdc_covid_vaccines_how_they_work / https://www.cdc.gov/covid/vaccines/how-they-work.html | strip_heading_anchors | 2495 | 0 |
| freeze_review:cdc_covid_vaccines_how_they_work / https://www.cdc.gov/covid/vaccines/how-they-work.html | label_only_internal_links | 2495 | 0 |
| freeze_review:cdc_covid_vaccines_how_they_work / https://www.cdc.gov/covid/vaccines/how-they-work.html | footnote_links | 2551 | 56 |
| freeze_review:japan_cherry_blossom_forecast / https://www.japan.travel/en/see-and-do/cherry-blossom-forecast/ | markdown | 814 | 0 |
| freeze_review:japan_cherry_blossom_forecast / https://www.japan.travel/en/see-and-do/cherry-blossom-forecast/ | aggressive_current | 814 | 0 |
| freeze_review:japan_cherry_blossom_forecast / https://www.japan.travel/en/see-and-do/cherry-blossom-forecast/ | drop_ui_lines | 814 | 0 |
| freeze_review:japan_cherry_blossom_forecast / https://www.japan.travel/en/see-and-do/cherry-blossom-forecast/ | strip_heading_anchors | 814 | 0 |
| freeze_review:japan_cherry_blossom_forecast / https://www.japan.travel/en/see-and-do/cherry-blossom-forecast/ | label_only_internal_links | 817 | 3 |
| freeze_review:japan_cherry_blossom_forecast / https://www.japan.travel/en/see-and-do/cherry-blossom-forecast/ | footnote_links | 822 | 8 |
