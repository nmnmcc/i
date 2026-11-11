(
	std/http
	std/list
	std/property
	i/core
	std/bit_array
	(Some, None) = std/option
)

@test
http_messages = () {
	input = <<
		"POST /a HTTP/1.1\r\nHost: example\r\nContent-Length: 5\r\nX-A: one\r\nx-a: two\r\n\r\nhelloREST":utf8
	>>
	http.Complete(req, rest) = http.feed_request(<<>>, input)
	assert(req.method == http.Method("POST"))
	assert(req.body == <<"hello":utf8>>)
	assert(rest == <<"REST":utf8>>)
	assert(http.headers(req.headers, "X-A") == ["one", "two"])
	assert(http.header(req.headers, "HOST") == Some("example"))
	http.Complete(req2, <<>>) = http.feed_request(
		<<>>
		http.serialize_request(req)
	)
	assert(req2 == req)
	chunk = <<
		"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nhi\r\n3\r\nbye\r\n0\r\nX-End: yes\r\n\r\n!":utf8
	>>
	http.Complete(res, tail) = http.feed_response(<<>>, chunk)
	assert(res.body == <<"hibye":utf8>>)
	assert(tail == <<"!":utf8>>)
	assert(http.header(res.trailers, "x-end") == Some("yes"))
	http.Complete(res2, <<>>) = http.feed_response(
		<<>>
		http.serialize_response(res)
	)
	assert(res2 == res)
	assert(http.parse_query("a=b+c") == Ok([("a", "b c")]))
	assert(http.build_query([("a", "b c")]) == "a=b+c")
}

@test
http_incremental = () {
	input = <<"POST / HTTP/1.1\r\nContent-Length: 4\r\n\r\ntest":utf8>>
	go = (rest: BitArray, buffered: BitArray) -> Nil ! Assert {
		rest |> {
			<<x, tail:bytes>> -> http.feed_request(buffered, <<x>>) |> {
				http.More(next) -> go(tail, next)
				http.Complete(req, left) -> {
					assert(tail == <<>>)
					assert(left == <<>>)
					assert(req.body == <<"test":utf8>>)
				}
				http.Invalid(_) -> assert(False)
			}
			_ -> assert(False)
		}
	}
	go(input, <<>>)
}

@test
http_rejects = () {
	bad = [
		"GET / HTTP/1.0\r\n\r\n"
		"GET  / HTTP/1.1\r\n\r\n"
		"G@T / HTTP/1.1\r\n\r\n"
		"GET / HTTP/1.1\n\n"
		"GET / HTTP/1.1\r\n Fold: x\r\n\r\n"
		"GET / HTTP/1.1\r\nBad Header: x\r\n\r\n"
		"GET / HTTP/1.1\r\nNoColon\r\n\r\n"
		"GET / HTTP/1.1\r\nContent-Length: -1\r\n\r\n"
		"GET / HTTP/1.1\r\nContent-Length: x\r\n\r\n"
		"GET / HTTP/1.1\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n"
		"GET / HTTP/1.1\r\nTransfer-Encoding: gzip\r\n\r\n"
		"GET / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\nG\r\n"
		"GET / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n1\r\nxXX"
	]
	list.each(bad, (s) {
		assert(http.feed_request(<<>>, <<s:utf8>>) |> {
			http.Invalid(_) -> True
			_               -> False
		})
	})
}

@test
http_head_cases = () {
	requests = [
		"GET / HTTP/1.1\r\n\r\n"
		"HEAD /x HTTP/1.1\r\nHost: h\r\n\r\n"
		"POST /x?q=a HTTP/1.1\r\nContent-Length: 0\r\n\r\n"
		"PUT /a HTTP/1.1\r\nContent-Length: 1\r\n\r\na"
		"DELETE / HTTP/1.1\r\nX: y\r\n\r\n"
		"OPTIONS * HTTP/1.1\r\nHost: h\r\n\r\n"
		"PATCH /p HTTP/1.1\r\nX: a\r\nX: b\r\n\r\n"
		"CONNECT h:80 HTTP/1.1\r\n\r\n"
		"TRACE / HTTP/1.1\r\nX: \t a \t\r\n\r\n"
		"custom / HTTP/1.1\r\nX-1: value\r\n\r\n"
		"GET / HTTP/1.1\r\nEmpty:\r\n\r\n"
		"GET / HTTP/1.1\r\nX: a:b\r\n\r\n"
		"GET / HTTP/1.1\r\nContent-Length: 3\r\n\r\nabc"
		"GET / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n"
		"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n1\r\na\r\n0\r\n\r\n"
	]
	list.each(requests, (s) {
		http.Complete(req, <<>>) = http.feed_request(<<>>, <<s:utf8>>)
		assert(
			http.feed_request(<<>>, http.serialize_request(req)) ==
				http.Complete(req, <<>>)
		)
	})
	responses = [
		"HTTP/1.1 200 OK\r\n\r\n"
		"HTTP/1.1 201 Created\r\nX: a\r\n\r\n"
		"HTTP/1.1 204 No Content\r\n\r\n"
		"HTTP/1.1 301 Moved Permanently\r\nLocation: /\r\n\r\n"
		"HTTP/1.1 302 Found\r\n\r\n"
		"HTTP/1.1 304 Not Modified\r\n\r\n"
		"HTTP/1.1 400 Bad Request\r\n\r\n"
		"HTTP/1.1 401 Unauthorized\r\n\r\n"
		"HTTP/1.1 403 Forbidden\r\n\r\n"
		"HTTP/1.1 404 Not Found\r\n\r\n"
		"HTTP/1.1 405 Method Not Allowed\r\n\r\n"
		"HTTP/1.1 500 Internal Server Error\r\n\r\n"
		"HTTP/1.1 502 Bad Gateway\r\n\r\n"
		"HTTP/1.1 503 Service Unavailable\r\n\r\n"
		"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi"
	]
	list.each(responses, (s) {
		http.Complete(res, <<>>) = http.feed_response(<<>>, <<s:utf8>>)
		assert(
			http.feed_response(<<>>, http.serialize_response(res)) ==
				http.Complete(res, <<>>)
		)
	})
}

@test
http_incremental_chunked_response = () {
	input = <<
		"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1\r\na\r\n0\r\nX: y\r\n\r\n":utf8
	>>
	go = (rest: BitArray, buffered: BitArray) -> Nil ! Assert {
		rest |> {
			<<x, tail:bytes>> -> http.feed_response(buffered, <<x>>) |> {
				http.More(next) -> go(tail, next)
				http.Complete(res, left) -> {
					assert(tail == <<>>)
					assert(left == <<>>)
					assert(res.body == <<"a":utf8>>)
					assert(http.header(res.trailers, "x") == Some("y"))
				}
				http.Invalid(_) -> assert(False)
			}
			_ -> assert(False)
		}
	}
	go(input, <<>>)
}

@test
http_requests_0_get_http_1_1_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<"GET / HTTP/1.1\r\n\r\n":utf8>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_requests_1_head_x_http_1_1_r_nhost_h_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<"HEAD /x HTTP/1.1\r\nHost: h\r\n\r\n":utf8>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_requests_2_post_x_q_a_http_1_1_r_ncontent_lengt = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<"POST /x?q=a HTTP/1.1\r\nContent-Length: 0\r\n\r\n":utf8>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_requests_3_put_a_http_1_1_r_ncontent_length_1_r = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<"PUT /a HTTP/1.1\r\nContent-Length: 1\r\n\r\na":utf8>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_requests_4_delete_http_1_1_r_nx_y_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<"DELETE / HTTP/1.1\r\nX: y\r\n\r\n":utf8>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_requests_5_options_http_1_1_r_nhost_h_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<"OPTIONS * HTTP/1.1\r\nHost: h\r\n\r\n":utf8>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_requests_6_patch_p_http_1_1_r_nx_a_r_nx_b_r_n_r = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<"PATCH /p HTTP/1.1\r\nX: a\r\nX: b\r\n\r\n":utf8>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_requests_7_connect_h_80_http_1_1_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<"CONNECT h:80 HTTP/1.1\r\n\r\n":utf8>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_requests_8_trace_http_1_1_r_nx_t_a_t_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<"TRACE / HTTP/1.1\r\nX: \t a \t\r\n\r\n":utf8>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_requests_9_custom_http_1_1_r_nx_1_value_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<"custom / HTTP/1.1\r\nX-1: value\r\n\r\n":utf8>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_requests_10_get_http_1_1_r_nempty_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<"GET / HTTP/1.1\r\nEmpty:\r\n\r\n":utf8>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_requests_11_get_http_1_1_r_nx_a_b_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<"GET / HTTP/1.1\r\nX: a:b\r\n\r\n":utf8>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_requests_12_get_http_1_1_r_ncontent_length_3_r_n = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<"GET / HTTP/1.1\r\nContent-Length: 3\r\n\r\nabc":utf8>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_requests_13_get_http_1_1_r_ntransfer_encoding_ch = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<"GET / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n":utf8>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_requests_14_post_http_1_1_r_ntransfer_encoding_c = () {
	http.Complete(value, <<>>) = http.feed_request(
		<<>>
		<<
			"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n1\r\na\r\n0\r\n\r\n":utf8
		>>
	)
	assert(
		http.feed_request(<<>>, http.serialize_request(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_0_http_1_1_200_ok_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 200 OK\r\n\r\n":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_1_http_1_1_201_created_r_nx_a_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 201 Created\r\nX: a\r\n\r\n":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_2_http_1_1_204_no_content_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 204 No Content\r\n\r\n":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_3_http_1_1_301_moved_permanently_r_nlo = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 301 Moved Permanently\r\nLocation: /\r\n\r\n":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_4_http_1_1_302_found_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 302 Found\r\n\r\n":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_5_http_1_1_304_not_modified_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 304 Not Modified\r\n\r\n":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_6_http_1_1_400_bad_request_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 400 Bad Request\r\n\r\n":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_7_http_1_1_401_unauthorized_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 401 Unauthorized\r\n\r\n":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_8_http_1_1_403_forbidden_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 403 Forbidden\r\n\r\n":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_9_http_1_1_404_not_found_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 404 Not Found\r\n\r\n":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_10_http_1_1_405_method_not_allowed_r_n_ = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 405 Method Not Allowed\r\n\r\n":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_11_http_1_1_500_internal_server_error_r = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 500 Internal Server Error\r\n\r\n":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_12_http_1_1_502_bad_gateway_r_n_r_n = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 502 Bad Gateway\r\n\r\n":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_13_http_1_1_503_service_unavailable_r_n = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 503 Service Unavailable\r\n\r\n":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_responses_14_http_1_1_200_ok_r_ncontent_length_2_ = () {
	http.Complete(value, <<>>) = http.feed_response(
		<<>>
		<<"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi":utf8>>
	)
	assert(
		http.feed_response(<<>>, http.serialize_response(value)) ==
			http.Complete(value, <<>>)
	)
}

@test
http_bad_0_get_http_1_0_r_n_r_n = () {
	assert(http.feed_request(<<>>, <<"GET / HTTP/1.0\r\n\r\n":utf8>>) |> {
		http.Invalid(_) -> True
		_               -> False
	})
}

@test
http_bad_1_get_http_1_1_r_n_r_n = () {
	assert(http.feed_request(<<>>, <<"GET  / HTTP/1.1\r\n\r\n":utf8>>) |> {
		http.Invalid(_) -> True
		_               -> False
	})
}

@test
http_bad_2_g_t_http_1_1_r_n_r_n = () {
	assert(http.feed_request(<<>>, <<"G@T / HTTP/1.1\r\n\r\n":utf8>>) |> {
		http.Invalid(_) -> True
		_               -> False
	})
}

@test
http_bad_3_get_http_1_1_n_n = () {
	assert(http.feed_request(<<>>, <<"GET / HTTP/1.1\n\n":utf8>>) |> {
		http.Invalid(_) -> True
		_               -> False
	})
}

@test
http_bad_4_get_http_1_1_r_n_fold_x_r_n_r_n = () {
	assert(
		http.feed_request(
			<<>>
			<<"GET / HTTP/1.1\r\n Fold: x\r\n\r\n":utf8>>
		) |> {
			http.Invalid(_) -> True
			_               -> False
		}
	)
}

@test
http_bad_5_get_http_1_1_r_nbad_header_x_r_n_r_n = () {
	assert(
		http.feed_request(
			<<>>
			<<"GET / HTTP/1.1\r\nBad Header: x\r\n\r\n":utf8>>
		) |> {
			http.Invalid(_) -> True
			_               -> False
		}
	)
}

@test
http_bad_6_get_http_1_1_r_nnocolon_r_n_r_n = () {
	assert(
		http.feed_request(
			<<>>
			<<"GET / HTTP/1.1\r\nNoColon\r\n\r\n":utf8>>
		) |> {
			http.Invalid(_) -> True
			_               -> False
		}
	)
}

@test
http_bad_7_get_http_1_1_r_ncontent_length_1_r_n = () {
	assert(
		http.feed_request(
			<<>>
			<<"GET / HTTP/1.1\r\nContent-Length: -1\r\n\r\n":utf8>>
		) |> {
			http.Invalid(_) -> True
			_               -> False
		}
	)
}

@test
http_bad_8_get_http_1_1_r_ncontent_length_x_r_n = () {
	assert(
		http.feed_request(
			<<>>
			<<"GET / HTTP/1.1\r\nContent-Length: x\r\n\r\n":utf8>>
		) |> {
			http.Invalid(_) -> True
			_               -> False
		}
	)
}

@test
http_bad_9_get_http_1_1_r_ncontent_length_0_r_n = () {
	assert(
		http.feed_request(
			<<>>
			<<
				"GET / HTTP/1.1\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n":utf8
			>>
		) |> {
			http.Invalid(_) -> True
			_               -> False
		}
	)
}

@test
http_bad_10_get_http_1_1_r_ntransfer_encoding_gz = () {
	assert(
		http.feed_request(
			<<>>
			<<"GET / HTTP/1.1\r\nTransfer-Encoding: gzip\r\n\r\n":utf8>>
		) |> {
			http.Invalid(_) -> True
			_               -> False
		}
	)
}

@test
http_bad_11_get_http_1_1_r_ntransfer_encoding_ch = () {
	assert(
		http.feed_request(
			<<>>
			<<"GET / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\nG\r\n":utf8>>
		) |> {
			http.Invalid(_) -> True
			_               -> False
		}
	)
}

@test
http_bad_12_get_http_1_1_r_ntransfer_encoding_ch = () {
	assert(
		http.feed_request(
			<<>>
			<<
				"GET / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n1\r\nxXX":utf8
			>>
		) |> {
			http.Invalid(_) -> True
			_               -> False
		}
	)
}

@test
http_close_delimited = () {
	input = <<"HTTP/1.1 200 OK\r\n\r\nabc":utf8>>
	http.More(buffer) = http.feed_response_for(
		<<>>
		input
		http.Method("GET")
		False
		1024
	)
	http.Complete(res, <<>>) = http.feed_response_for(
		buffer
		<<>>
		http.Method("GET")
		True
		1024
	)
	assert(res.body == <<"abc":utf8>>)
}

@test
http_response_head_method = () {
	input = <<"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nNEXT":utf8>>
	http.Complete(res, rest) = http.feed_response_for(
		<<>>
		input
		http.Method("HEAD")
		False
		1024
	)
	assert(res.body == <<>>)
	assert(rest == <<"NEXT":utf8>>)
}

@test
http_response_connect = () {
	input = <<"HTTP/1.1 200 OK\r\n\r\nNEXT":utf8>>
	http.Complete(res, rest) = http.feed_response_for(
		<<>>
		input
		http.Method("CONNECT")
		False
		1024
	)
	assert(res.body == <<>>)
	assert(rest == <<"NEXT":utf8>>)
}

@test
http_limit_head = () {
	input = <<"GET / HTTP/1.1\r\n\r\n":utf8>>
	http.Invalid(_) = http.feed_request_limit(<<>>, input, 8)
	Nil
}

@test
http_equal_content_lengths = () {
	input = <<
		"POST / HTTP/1.1\r\nContent-Length: 1\r\nContent-Length: 1\r\n\r\na":utf8
	>>
	http.Complete(req, <<>>) = http.feed_request(<<>>, input)
	assert(req.body == <<"a":utf8>>)
}

@test
http_bad_chunk_extension = () {
	input = <<
		"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n1;=x\r\na\r\n0\r\n\r\n":utf8
	>>
	http.Invalid(_) = http.feed_request(<<>>, input)
	Nil
}

@test
http_obs_text_round_trip = () {
	input = <<"GET / HTTP/1.1\r\nX: ":utf8, 255, "\r\n\r\n":utf8>>
	http.Complete(req, <<>>) = http.feed_request(<<>>, input)
	assert(http.header_bytes(req.headers, "x") == Some(<<255>>))
	assert(http.serialize_request(req) == input)
}

@test
http_generated_body_round_trip = () {
	Ok(gen) = property.string(30)
	property.test(gen, property.Config(17, 100, 30, 0, 100), (s) {
		body = <<s:utf8>>
		req = http.Request(
			http.Method("POST")
			"/"
			http.Headers([])
			body
			http.Headers([])
		)
		http.Complete(parsed, <<>>) = http.feed_request(
			<<>>
			http.serialize_request(req)
		)
		assert(parsed.body == body)
	}, core.inspect)
}

@test
http_quoted_chunk_extension = () {
	input = <<
		"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n1; foo=\"a\\\"b\"\r\na\r\n0\r\nX-End: yes\r\n\r\n":utf8
	>>
	http.Complete(req, <<>>) = http.feed_request(<<>>, input)
	assert(req.body == <<"a":utf8>>)
	assert(http.header(req.trailers, "X-End") == Some("yes"))
}

@test
http_forbidden_trailer = () {
	input = <<
		"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nContent-Length: 1\r\n\r\n":utf8
	>>
	http.Invalid(_) = http.feed_request(<<>>, input)
	Nil
}

@test
http_content_length_list = () {
	input = <<"POST / HTTP/1.1\r\nContent-Length: 01, 1\r\n\r\na":utf8>>
	http.Complete(req, <<>>) = http.feed_request(<<>>, input)
	assert(req.body == <<"a":utf8>>)
}

@test
http_transfer_coding_chain = () {
	input = <<
		"POST / HTTP/1.1\r\nTransfer-Encoding: gzip, chunked\r\n\r\n1\r\na\r\n0\r\n\r\n":utf8
	>>
	http.Complete(req, <<>>) = http.feed_request(<<>>, input)
	assert(req.body == <<"a":utf8>>)
	assert(
		http.feed_request(<<>>, http.serialize_request(req)) ==
			http.Complete(req, <<>>)
	)
}

@test
http_response_transfer_close = () {
	input = <<"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip\r\n\r\nabc":utf8>>
	http.More(buffer) = http.feed_response_for(
		<<>>
		input
		http.Method("GET")
		False
		100
	)
	http.Complete(res, <<>>) = http.feed_response_for(
		buffer
		<<>>
		http.Method("GET")
		True
		100
	)
	assert(res.body == <<"abc":utf8>>)
}

@test
http_reject_chunk_size_leading_space = () {
	input = <<
		"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n 1\r\na\r\n0\r\n\r\n":utf8
	>>
	http.Invalid(_) = http.feed_request(<<>>, input)
	Nil
}

@test
http_reject_chunk_extension_trailing_space = () {
	input = <<
		"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n1;x \r\na\r\n0\r\n\r\n":utf8
	>>
	http.Invalid(_) = http.feed_request(<<>>, input)
	Nil
}

@test
provider_and_route = () {
	provider = (send: (req: http.Request) { http.response(http.Status(201)) })
	res = http.send_with(
		provider
		http.request(http.Method("GET"), "http://example.com")
	)
	assert(res.status == http.Status(201))
	assert(
		http.route(
			http.Method("GET")
			"/users/:id"
			http.request(http.Method("GET"), "/users/42")
		) ==
			Some([("id", "42")])
	)
	assert(
		http.route(
			http.Method("POST")
			"/users/:id"
			http.request(http.Method("GET"), "/users/42")
		) ==
			None
	)
}
