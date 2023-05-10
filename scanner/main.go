package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strconv"
	"time"
)

type ListWaypointResponse struct {
  Symbol string
  Type string
  SystemSymbol string
  X int
  Y int
  Traits []struct {
    Symbol string
    Name string
    Description string
  }
}

type ListSystemResponse struct {
  Symbol string
  SectorSymbol string
  Type string
  X int
  Y int
  Waypoints []struct {
    Symbol string
    Type string
    X int
    Y int
  }
  Factions []struct {
    Symbol string
  }
}

type GetMarketResponse struct {
  Exports []struct {
    Symbol string
    Name string
    Description string
  }
  Imports []struct {
    Symbol string
    Name string
    Description string
  }
  Exchange []struct {
    Symbol string
    Name string
    Description string
  }
}

type ResponseWithoutMeta[T any] struct {
  Data T
}

type Response[T any] struct {
  Data T
  Meta struct {
    Page int
    Total int
    Limit int
  }
}

func get[T any](url string, q url.Values) Response[T] {
  req, err := http.NewRequest("GET", url, nil)
  if err != nil {
    panic(err)
  }

  req.Header.Add("Authorization", "Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJpZGVudGlmaWVyIjoiU09JRVMiLCJpYXQiOjE2ODM1OTgwMzQsInN1YiI6ImFnZW50LXRva2VuIn0.GifA8eeFcatu05P6TAetT9SzuaBX-37eeFwYN26-NTHc1eobzsah6YBmYm9QEmfVpb5GBWNxv80Pz7C6XmUxWVNXKKeQoC5C_sZRIW-WlWQMa6Ek1kSYy6-PDwHnFBdbP4UbG3_KJmWIUZZZrmqqGaQr72XCTKxFipvWGAvBGx9ogtazKLo0gvCRvFVM6Egs_Em8MgZqog0ixeRXTATjgEHD9QW_OWsb-X2bEm7Bqut2l27cm3QWFHXw8ZXhTaxqODYCg2XCN90owG8TlLEL7thcfhiOL9n2DL12F3tU8_YqdZ6XBgQy4ujAWO_y5N1RrL4MEuCQw1s83vVbYlljt3UVhnf6sn47thmZhGEjk4xXa8jq_X2fa2LTTuYc2grjCYACN_VRg9Lfc_w4CpWM4aF0wyYnmhEGccjpg7ywkvqhzswYQufB-z83uqLRPm--wn4DJcW6dCrLzEf6tdTCCcgfA0O3F-ISIh9j-6H7S9x_gx_gnWO73bGIK7QUVvQe")
  que := req.URL.Query()
  for key, values := range q {
    for _, value := range values {
      que.Add(key, value)
    }
  }

  req.URL.RawQuery = que.Encode()

  client := new(http.Client)

  res, err := client.Do(req)
  if err != nil {
    panic(err)
  }

  defer res.Body.Close()

  var value Response[T]

  body, _ := io.ReadAll(res.Body)
  if err := json.Unmarshal(body, &value); err != nil {
    panic(err)
  }

  time.Sleep(time.Millisecond * 500)

  return value
}

func main() {
  for i := 0; i < 2; i++ {
    q := url.Values{}
    q.Add("page", strconv.Itoa(i + 1))

    systems := get[[]ListSystemResponse]("https://api.spacetraders.io/v2/systems", q)

    for _, system := range systems.Data {
      points := get[[]ListWaypointResponse](fmt.Sprintf("https://api.spacetraders.io/v2/systems/%s/waypoints", system.Symbol), url.Values{})

      for _, point := range points.Data {
        isMarket := false
        for _, trait := range point.Traits {
          if trait.Symbol == "MARKETPLACE" {
            isMarket = true
          }
          if trait.Symbol == "BLACK_MARKET" {
            isMarket = true
          }
        }

        if isMarket {
          market := get[GetMarketResponse](fmt.Sprintf("https://api.spacetraders.io/v2/systems/%s/waypoints/%s/market", system.Symbol, point.Symbol), url.Values{})
          fmt.Printf("----------- %s -----------\n", point.Symbol)
          fmt.Println("IMPORTS")
          for _, good := range market.Data.Imports {
            fmt.Printf("\t%s\n", good.Symbol)
          }
          fmt.Println("EXPORTS")
          for _, good := range market.Data.Exports {
            fmt.Printf("\t%s\n", good.Symbol)
          }
          fmt.Println("EXCHANGES")
          for _, good := range market.Data.Exchange {
            fmt.Printf("\t%s\n", good.Symbol)
          }
          fmt.Println()
        }
      }
    }
  }
}
