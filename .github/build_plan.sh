#! /bin/bash

matrix_file=./.github/build_matrix.json

should_publish=$([ $IS_NEW_RELEASE = true ])

if [ $IS_MERGED_PR = true ] && [ $IS_BETA_BRANCH = true ]; then
    should_publish=true
fi

if [ $should_publish = true ]; then
    include_filter='.config'
    exclude_filter='[]'
else
    include_filter='[.config[] | select(.target == "x86_64-unknown-linux-gnu")]'
    exclude_filter='[.config[] | select(.target != "x86_64-unknown-linux-gnu")]'
fi

included=$(jq -c "{ package, config: $include_filter }" < "$matrix_file")
excluded=$(jq -c "{ package, config: $exclude_filter }" < "$matrix_file")

echo "build_matrix=${included}" >> $GITHUB_OUTPUT
echo "should_publish=${should_publish}" >> $GITHUB_OUTPUT

echo "Is Release Branch: $IS_RELEASE_BRANCH"
echo "Is Beta Branch: $IS_BETA_BRANCH"
echo "Should Publish: $should_publish"

echo -e "\n== Included =="
echo $included | jq .config

echo -e "\n== Excluded =="
echo $excluded | jq .config