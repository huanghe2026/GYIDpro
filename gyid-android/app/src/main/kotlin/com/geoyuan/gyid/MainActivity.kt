package com.geoyuan.gyid

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.navigation.NavDestination.Companion.hierarchy
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import com.geoyuan.gyid.ui.CollectScreen
import com.geoyuan.gyid.ui.IdentityScreen
import com.geoyuan.gyid.ui.VerifyScreen

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                GyidApp()
            }
        }
    }
}

private enum class GyidRoute(val label: String) {
    Identity("身份"),
    Collect("采集"),
    Verify("验证"),
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun GyidApp() {
    val navController = rememberNavController()
    Scaffold(
        bottomBar = {
            NavigationBar {
                val backStack by navController.currentBackStackEntryAsState()
                val current = backStack?.destination
                GyidRoute.entries.forEach { route ->
                    NavigationBarItem(
                        selected = current?.hierarchy?.any { it.route == route.name } == true,
                        onClick = {
                            navController.navigate(route.name) {
                                launchSingleTop = true
                            }
                        },
                        label = { Text(route.label) },
                        icon = { Text(route.label.take(1)) },
                    )
                }
            }
        },
    ) { padding ->
        NavHost(
            navController = navController,
            startDestination = GyidRoute.Identity.name,
            modifier = Modifier.padding(padding),
        ) {
            composable(GyidRoute.Identity.name) { IdentityScreen() }
            composable(GyidRoute.Collect.name) { CollectScreen() }
            composable(GyidRoute.Verify.name) { VerifyScreen() }
        }
    }
}
