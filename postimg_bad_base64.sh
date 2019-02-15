IMG="fakedbase64data"
curl -X POST -i -H "Content-Type: application/json" -d '{"images":["'$IMG'"], "urls":["https://3dnews.ru/assets/external/icons/2019/02/11/982599.jpg"]}' http://localhost:3000/upload

