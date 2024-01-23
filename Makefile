rabbitmq_creds:
	echo User: $(shell kubectl -n ao3 get secret message-broker-default-user -o jsonpath="{.data.username}" | base64 --decode)
	echo Pass: $(shell kubectl -n ao3 get secret message-broker-default-user -o jsonpath="{.data.password}" | base64 --decode)
	kubectl rabbitmq -n ao3 manage message-broker
