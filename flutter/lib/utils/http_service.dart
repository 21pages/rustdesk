import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:flutter_hbb/consts.dart';
import 'package:http/http.dart' as http;
import '../models/platform_model.dart';
import 'package:flutter_hbb/common.dart';
export 'package:http/http.dart' show Response;

enum HttpMethod { get, post, put, delete }

class HttpService {
  Future<http.Response> sendRequest(
    Uri url,
    HttpMethod method, {
    Map<String, String>? headers,
    dynamic body,
  }) async {
    headers ??= {'Content-Type': 'application/json'};

    // Use Rust HTTP implementation for non-web platforms for consistency.
    var useFlutterHttp = (isWeb || kIsWeb);
    if (!useFlutterHttp) {
      final enableFlutterHttpOnRust =
          mainGetLocalBoolOptionSync(kOptionEnableFlutterHttpOnRust);
      // Use flutter http if:
      // Not `enableFlutterHttpOnRust` and no proxy is set
      useFlutterHttp =
          !(enableFlutterHttpOnRust || await bind.mainGetProxyStatus());
    }

    if (useFlutterHttp) {
      return await _pollFlutterHttp(url, method, headers: headers, body: body);
    }

    String headersJson = jsonEncode(headers);
    String methodName = method.toString().split('.').last;
    logToRust(
        "http service sendRequest: $url, $methodName, $body, $headersJson");
    await bind.mainHttpRequest(
        url: url.toString(),
        method: methodName.toLowerCase(),
        body: body,
        header: headersJson);

    logToRust("http service sendRequest after mainHttpRequest");
    var resJson = await _pollForResponse(url.toString());
    logToRust("http service sendRequest after pollForResponse: $resJson");
    var res = _parseHttpResponse(resJson);
    logToRust("http service sendRequest after parseHttpResponse: $res");
    return res;
  }

  Future<http.Response> _pollFlutterHttp(
    Uri url,
    HttpMethod method, {
    Map<String, String>? headers,
    dynamic body,
  }) async {
    var response = http.Response('', 400);

    switch (method) {
      case HttpMethod.get:
        response = await http.get(url, headers: headers);
        break;
      case HttpMethod.post:
        response = await http.post(url, headers: headers, body: body);
        break;
      case HttpMethod.put:
        response = await http.put(url, headers: headers, body: body);
        break;
      case HttpMethod.delete:
        response = await http.delete(url, headers: headers, body: body);
        break;
      default:
        throw Exception('Unsupported HTTP method');
    }

    return response;
  }

  Future<String> _pollForResponse(String url) async {
    logToRust("http service _pollForResponse: $url");
    String? responseJson = " ";
    while (responseJson == " ") {
      responseJson = await bind.mainGetHttpStatus(url: url);
      if (responseJson == null) {
        logToRust("http service _pollForResponse responseJson is null");
        throw Exception('The HTTP request failed');
      }
      if (responseJson == " ") {
        await Future.delayed(const Duration(milliseconds: 100));
      }
    }
    logToRust("http service _pollForResponse responseJson: $responseJson");
    return responseJson!;
  }

  http.Response _parseHttpResponse(String responseJson) {
    try {
      logToRust("http service _parseHttpResponse: $responseJson");
      var parsedJson = jsonDecode(responseJson);
      logToRust("http service _parseHttpResponse parsedJson: $parsedJson");
      String body = parsedJson['body'];
      logToRust("http service _parseHttpResponse body: $body");
      Map<String, String> headers = {};
      logToRust(
          "http service _parseHttpResponse headers: ${parsedJson['headers']}");
      for (var key in parsedJson['headers'].keys) {
        headers[key] = parsedJson['headers'][key];
      }
      int statusCode = parsedJson['status_code'];
      logToRust("http service _parseHttpResponse statusCode: $statusCode");
      return http.Response(body, statusCode, headers: headers);
    } catch (e) {
      print('Failed to parse response\n$responseJson\nError:\n$e');
      throw Exception('Failed to parse response.\n$responseJson');
    }
  }
}

Future<http.Response> get(Uri url, {Map<String, String>? headers}) async {
  return await HttpService().sendRequest(url, HttpMethod.get, headers: headers);
}

Future<http.Response> post(Uri url,
    {Map<String, String>? headers, Object? body, Encoding? encoding}) async {
  return await HttpService()
      .sendRequest(url, HttpMethod.post, body: body, headers: headers);
}

Future<http.Response> put(Uri url,
    {Map<String, String>? headers, Object? body, Encoding? encoding}) async {
  return await HttpService()
      .sendRequest(url, HttpMethod.put, body: body, headers: headers);
}

Future<http.Response> delete(Uri url,
    {Map<String, String>? headers, Object? body, Encoding? encoding}) async {
  return await HttpService()
      .sendRequest(url, HttpMethod.delete, body: body, headers: headers);
}
