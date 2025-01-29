#! /bin/bash

matrix_file=./.github/build_matrix.json

if [ $IS_RELEASE = true ] || [ $IS_BETA = true ]; then
    include_filter='.config'
    exclude_filter='[]'
    should_publish=true
else
    include_filter='[.config[] | select(.arch == "x86_64")]'
    exclude_filter='[.config[] | select(.arch != "x86_64")]'
    should_publish=false
fi

included=$(jq -c "{ package, config: $include_filter }" < "$matrix_file")
excluded=$(jq -c "{ package, config: $exclude_filter }" < "$matrix_file")

echo "build_matrix=${included}" >> $GITHUB_OUTPUT
echo "should_publish=${should_publish}" >> $GITHUB_OUTPUT

echo "Is Release: $IS_RELEASE"
echo "Is Beta: $IS_BETA"
echo "Should Publish: $should_publish"

echo -e "\n== Included =="
echo $included | jq .config

echo -e "\n== Excluded =="
echo $excluded | jq .config
