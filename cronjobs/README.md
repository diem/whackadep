# Cronjobs

HOW DO WE MAKE IT SO THAT WE CAN TRIGGER THIS MANUALLY FROM THE UI AS WELL?

Here are the metrics that we want to collect.

## Daily

update, if there's any changes, to:

* list of our direct dependencies (& metadata)
* list of our indirect dependencies (& metadata)
* metrics on our current dependencies

retrieve information on:

* list of our transitive (direct & indirect) dependencies that have new versions
* metrics on these changes (red flags?)

## Hourly

* cargo-audit result to see if any RUST-SEC is relevant on our current dependencies.
