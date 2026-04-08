# Frozen Fixture Status

This directory stores local HTML snapshots promoted from reviewed seed URLs.

Use `cargo run --example prepare_freeze_targets` after editing `corpus/seeds/freeze_review.csv`.

- accepted rows: 8
- ready targets: 8
- already frozen locally: 7
- missing local html: 1
- missing fixture metadata: 0

## Bucket Summary

| bucket | accepted targets | frozen locally |
| --- | ---: | ---: |
| finance | 1 | 1 |
| health | 1 | 1 |
| legal | 1 | 1 |
| product | 2 | 2 |
| programming | 1 | 0 |
| science | 1 | 1 |
| travel | 1 | 1 |

## Ready Targets

| fixture | bucket | local_state | review_fetch_status | url |
| --- | --- | --- | --- | --- |
| irs_traditional_and_roth_iras | finance | frozen | frozen | https://www.irs.gov/retirement-plans/traditional-and-roth-iras |
| harvard_gut_health | health | frozen | frozen | https://www.health.harvard.edu/healthy-aging-and-longevity/5-simple-ways-to-improve-gut-health |
| texas_small_claims_filing | legal | frozen | frozen | https://guides.sll.texas.gov/small-claims/filing-information |
| target_nutrition_now_pb8_capsules | product | frozen | frozen | https://www.target.com/p/-/A-90313128 |
| walmart_ip_man_box_set_blu_ray | product | frozen | failed | https://www.walmart.com/ip/160317419 |
| wikipedia_large_language_model | programming | missing_local_html | failed | https://en.wikipedia.org/wiki/Large_language_model |
| cdc_covid_vaccines_how_they_work | science | frozen | frozen | https://www.cdc.gov/covid/vaccines/how-they-work.html |
| japan_cherry_blossom_forecast | travel | frozen | frozen | https://www.japan.travel/en/see-and-do/cherry-blossom-forecast/ |
