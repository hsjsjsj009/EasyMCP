stdin_content=$(cat)
param1=$1
param2=$2

empty_json="{\"stdin\":\"\",\"param1\":\"\",\"param2\":\"\"}"

empty_json_new=$(echo $empty_json | jq --arg stdin "${stdin_content}" --arg param1 "${param1}" --arg param2 "${param2}" '.stdin=$stdin | .param1=$param1 | .param2=$param2')

echo "${empty_json_new}"
